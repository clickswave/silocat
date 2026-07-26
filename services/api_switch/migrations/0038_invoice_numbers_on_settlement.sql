-- Invoice numbers belong to settled orders only.
--
-- 0035 assigned the number on INSERT, so every abandoned checkout burned one.
-- That breaks the thing an invoice series exists to provide: under Rule 46 of
-- the CGST Rules the serial number must be consecutive within a financial year,
-- and a sequence where most entries correspond to no supply at all is neither
-- consecutive nor defensible in an audit.
--
-- The number is now assigned at the moment an order settles, and only then.

-- Assign when an order reaches a settled state and has no number yet. Covers
-- both the UPDATE path (pending order later paid) and the INSERT path (paths
-- that record an already-settled order in one statement).
CREATE OR REPLACE FUNCTION assign_invoice_number() RETURNS TRIGGER AS $$
DECLARE
    yr int;
    next_seq int;
BEGIN
    IF NEW.invoice_number IS NOT NULL THEN
        RETURN NEW;
    END IF;

    IF LOWER(COALESCE(NEW.status, '')) NOT IN ('paid', 'completed', 'success') THEN
        RETURN NEW;
    END IF;

    yr := EXTRACT(YEAR FROM COALESCE(NEW.created_on, NOW()))::int;

    SELECT COALESCE(MAX(SUBSTRING(invoice_number FROM '[0-9]{4}$')::int), 0) + 1
      INTO next_seq
      FROM orders
     WHERE invoice_number LIKE 'INV-' || yr || '-%';

    NEW.invoice_number := 'INV-' || yr || '-' || LPAD(next_seq::text, 4, '0');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_assign_invoice_number ON orders;
CREATE TRIGGER trg_assign_invoice_number
    BEFORE INSERT OR UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION assign_invoice_number();

-- Release the numbers held by orders that never settled.
UPDATE orders
   SET invoice_number = NULL
 WHERE invoice_number IS NOT NULL
   AND LOWER(COALESCE(status, '')) NOT IN ('paid', 'completed', 'success');

-- Close the gaps those left behind. Settled orders are renumbered in the order
-- they were placed, per year, so the surviving series is consecutive from 0001.
-- Done in two passes because the unique index would collide mid-renumber.
UPDATE orders SET invoice_number = 'TMP-' || reference_id
 WHERE invoice_number IS NOT NULL;

WITH numbered AS (
    SELECT reference_id,
           EXTRACT(YEAR FROM created_on)::int AS yr,
           ROW_NUMBER() OVER (
               PARTITION BY EXTRACT(YEAR FROM created_on)
               ORDER BY created_on, reference_id
           ) AS seq
    FROM orders
    WHERE invoice_number LIKE 'TMP-%'
)
UPDATE orders o
   SET invoice_number = 'INV-' || n.yr || '-' || LPAD(n.seq::text, 4, '0')
  FROM numbered n
 WHERE o.reference_id = n.reference_id;
