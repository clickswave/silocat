-- One definition of "this order was actually paid for".
--
-- The set lived in three places: the invoice trigger, the order-history query
-- and the frontend. Three copies of a rule that decides whether someone sees a
-- receipt is how a fourth gateway status quietly means "paid here, unpaid
-- there". This makes the database the authority; the frontend copy stays only
-- as a display guard, since the server no longer sends unsettled orders at all.

CREATE OR REPLACE FUNCTION order_is_settled(status text) RETURNS boolean AS $$
    SELECT LOWER(COALESCE(status, '')) IN ('paid', 'completed', 'success');
$$ LANGUAGE sql IMMUTABLE;

COMMENT ON FUNCTION order_is_settled(text) IS
    'True when an order status means money changed hands. Single source of truth: '
    'the invoice-number trigger, the order-history query and watchcat all use it.';

CREATE OR REPLACE FUNCTION assign_invoice_number() RETURNS TRIGGER AS $$
DECLARE
    prefix text;
    next_seq int;
BEGIN
    IF NEW.invoice_number IS NOT NULL THEN
        RETURN NEW;
    END IF;

    IF NOT order_is_settled(NEW.status) THEN
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
