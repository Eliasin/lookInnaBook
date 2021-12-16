-- Creates restock order for books if change to book stock lowers book below threshold and there
-- is not already a restock order placed that is pending
CREATE OR REPLACE FUNCTION public.restock_book_trigger()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE restock_exists INTEGER;
DECLARE last_month_sales INTEGER;
BEGIN

restock_exists := 0;
SELECT count("restock_order_id") INTO restock_exists FROM base.restock_order
WHERE order_status = 'PENDING';

last_month_sales := 0;
SELECT sum(quantity) INTO last_month_sales FROM base.orders INNER JOIN base.in_order USING (order_id) WHERE order_date > NOW() - interval '1 month' AND base.in_order.isbn = NEW.isbn;

IF NEW.stock <= NEW.reorder_threshold AND restock_exists < 1 THEN
INSERT INTO base.restock_order
(isbn, quantity, price_per_unit, order_date, order_status)
VALUES (NEW.isbn, last_month_sales, NEW.price * 0.7, NOW(), 'PENDING');

END IF;

RETURN NEW;


END;
$function$
