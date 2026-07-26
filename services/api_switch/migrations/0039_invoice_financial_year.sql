-- Invoice series keyed on the financial year, not the calendar year.
--
-- GST requires the serial number to be unique within a financial year, and the
-- Indian financial year runs April to March. A calendar-year series puts an
-- invoice from July 2026 and one from February 2027 in the same run while they
-- belong to two different financial years, which is exactly the boundary an
-- auditor reconciles against.
--
-- Format: INV-<fy start>-<fy end, 2 digits>-<4-digit sequence within that year>
-- e.g. INV-2026-27-0001 for anything issued between 2026-04-01 and 2027-03-31.
--
-- Safe to renumber here: nothing has been sold, so no invoice exists outside
-- this database. That stops being true the moment it does.

-- The financial year an instant belongs to, as its starting calendar year.
CREATE OR REPLACE FUNCTION invoice_fy_start(ts timestamptz) RETURNS int AS $$
    SELECT EXTRACT(YEAR FROM ts)::int
         - CASE WHEN EXTRACT(MONTH FROM ts)::int < 4 THEN 1 ELSE 0 END;
$$ LANGUAGE sql IMMUTABLE;

-- The series prefix for that financial year, e.g. 'INV-2026-27'.
CREATE OR REPLACE FUNCTION invoice_fy_prefix(ts timestamptz) RETURNS text AS $$
    SELECT 'INV-' || invoice_fy_start(ts)
        || '-' || LPAD(((invoice_fy_start(ts) + 1) % 100)::text, 2, '0');
$$ LANGUAGE sql IMMUTABLE;

CREATE OR REPLACE FUNCTION assign_invoice_number() RETURNS TRIGGER AS $$
DECLARE
    prefix text;
    next_seq int;
BEGIN
    IF NEW.invoice_number IS NOT NULL THEN
        RETURN NEW;
    END IF;

    IF LOWER(COALESCE(NEW.status, '')) NOT IN ('paid', 'completed', 'success') THEN
        RETURN NEW;
    END IF;

    prefix := invoice_fy_prefix(COALESCE(NEW.created_on, NOW()));

    SELECT COALESCE(MAX(SUBSTRING(invoice_number FROM '[0-9]{4}$')::int), 0) + 1
      INTO next_seq
      FROM orders
     WHERE invoice_number LIKE prefix || '-%';

    NEW.invoice_number := prefix || '-' || LPAD(next_seq::text, 4, '0');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_assign_invoice_number ON orders;
CREATE TRIGGER trg_assign_invoice_number
    BEFORE INSERT OR UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION assign_invoice_number();

-- Restate the existing settled orders under the financial-year series. Two
-- passes so the unique index cannot collide partway through.
UPDATE orders SET invoice_number = 'TMP-' || reference_id
 WHERE invoice_number IS NOT NULL;

WITH numbered AS (
    SELECT reference_id,
           invoice_fy_prefix(created_on) AS prefix,
           ROW_NUMBER() OVER (
               PARTITION BY invoice_fy_start(created_on)
               ORDER BY created_on, reference_id
           ) AS seq
    FROM orders
    WHERE invoice_number LIKE 'TMP-%'
)
UPDATE orders o
   SET invoice_number = n.prefix || '-' || LPAD(n.seq::text, 4, '0')
  FROM numbered n
 WHERE o.reference_id = n.reference_id;
