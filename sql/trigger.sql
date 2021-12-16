-- Creates restock orders after change to books if necessary
CREATE TRIGGER restock_books
AFTER UPDATE ON base.book
FOR EACH ROW EXECUTE FUNCTION public.restock_book_trigger();
