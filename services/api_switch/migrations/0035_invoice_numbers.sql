-- Invoices need a stable, human-readable number.
--
-- The Billing screen offers a downloadable invoice per order, and people file
-- those for tax. A number derived on the fly (say, from row position) would
-- silently renumber every historical invoice the moment an order is inserted or
-- refunded, so the number is stored once and never recomputed.
--
-- Format: INV-<year>-<4-digit sequence within that year>, e.g. INV-2026-0142.

ALTER TABLE orders ADD COLUMN IF NOT EXISTS invoice_number TEXT;

-- Backfill in creation order, per year, so existing receipts get numbers that
-- match the order they were actually placed in.
WITH numbered AS (
    SELECT reference_id,
           EXTRACT(YEAR FROM created_on)::int AS yr,
           ROW_NUMBER() OVER (
               PARTITION BY EXTRACT(YEAR FROM created_on)
               ORDER BY created_on, reference_id
           ) AS seq
    FROM orders
    WHERE invoice_number IS NULL
)
UPDATE orders o
SET invoice_number = 'INV-' || n.yr || '-' || LPAD(n.seq::text, 4, '0')
FROM numbered n
WHERE o.reference_id = n.reference_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_invoice_number
    ON orders (invoice_number) WHERE invoice_number IS NOT NULL;

-- New orders are created from several code paths (plan purchase, add-on, promo
-- settlement), so the number is assigned by a trigger rather than at each call
-- site: one place to be correct, and impossible to forget.
CREATE OR REPLACE FUNCTION assign_invoice_number() RETURNS TRIGGER AS $$
DECLARE
    yr int;
    next_seq int;
BEGIN
    IF NEW.invoice_number IS NOT NULL THEN
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
    BEFORE INSERT ON orders
    FOR EACH ROW EXECUTE FUNCTION assign_invoice_number();
