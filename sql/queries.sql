-- Getting data about all books
SELECT * FROM base.book;

-- Getting data about books with their publisher name
SELECT
isbn,
title,
author_name,
genre,
base.book.publisher_id,
company_name AS publisher_name,
num_pages,
price,
author_royalties,
reorder_threshold,
stock,
discontinued,
company_name AS publisher_name
FROM base.book INNER JOIN base.publisher ON base.book.publisher_id = base.publisher.publisher_id;

-- Getting customers with a certain email
SELECT customer_id, password_hash FROM base.customer WHERE email = $1;

-- Inserting new address into relation returning the ID of the new address
INSERT INTO base.address (street_address, postal_code, province) VALUES ($1, $2, $3) RETURNING address_id;

-- Insert new payment info into relation
INSERT INTO base.payment_info (name_on_card, expiry, card_number, cvv, billing_address_id) VALUES ($1, $2, $3, $4, $5) RETURNING payment_info_id;

-- Selects owner with certain email to see if owner exists with that email
SELECT * FROM base.owner WHERE email = $1;

-- Selects customer with certain email to see if customer exists with that email
SELECT * FROM base.customer WHERE email = $1;

-- Selects publisher with certain email to see if customer exists with that email
SELECT * FROM base.publisher WHERE email = $1

-- Creates new customer returning the ID of the new customer
INSERT INTO base.customer (name, email, password_hash, default_shipping_address, default_payment_info_id) VALUES ($1, $2, $3, $4, $5) RETURNING customer_id;

-- Creates new owner returning the ID of the new owner
INSERT INTO base.owner (name, email, password_hash) VALUES ($1, $2, $3) RETURNING owner_id;

-- Gets all owners to see if at least one exists (default owner login is enabled if none exist)
SELECT * FROM base.owner;

-- Gets owner login info for a certain email login
SELECT owner_id, password_hash FROM base.owner WHERE email = $1;

-- Get the basic details of a customer (ids not joined)
SELECT name, email, default_shipping_address, default_payment_info_id
FROM base.customer WHERE customer_id = $1;

-- Get the full details of a customer
SELECT
customer_id,
name,
email,
expiry,
name_on_card,
def_shipping.street_address AS def_street_address,
def_shipping.postal_code AS def_postal,
def_shipping.province AS def_province,
billing_add.street_address AS bill_street_address,
billing_add.postal_code AS bill_postal,
billing_add.province AS bill_province
FROM
base.customer AS customer
INNER JOIN base.address AS def_shipping ON customer.default_shipping_address = def_shipping.address_id
INNER JOIN base.payment_info AS payment ON customer.default_payment_info_id = payment.payment_info_id
INNER JOIN base.address AS billing_add ON payment.billing_address_id = billing_add.address_id
WHERE customer.customer_id = $1;

-- Get the contents of a customer's cart
SELECT isbn, quantity FROM base.in_cart WHERE customer_id = $1;

-- Get whether a book is discontinued or not
SELECT discontinued FROM base.book WHERE isbn = $1;

-- Get the quantity of a book in the cart
SELECT quantity FROM base.in_cart WHERE customer_id = $1 AND isbn = $2;

-- Add the quantity of a book in the cart
UPDATE base.in_cart SET quantity = quantity + 1 WHERE isbn = $1 AND customer_id = $2;

-- Add a new book to the cart
INSERT INTO base.in_cart (isbn, customer_id, quantity) VALUES ($1, $2, 1);

-- Get how much stock a book has
SELECT stock FROM base.book WHERE isbn = $1;

-- Remove a book from the cart
DELETE FROM base.in_cart WHERE isbn = $1 AND customer_id = $2;

-- Change the amount of a book in the cart
UPDATE base.in_cart SET quantity = $1 WHERE isbn = $2 AND customer_id = $3;

-- Find the ID of an address if it already exists
SELECT address_id FROM base.address WHERE street_address = $1 AND postal_code = $2 AND province = $3;


-- Find the ID of a payment information entity if it already exists
SELECT payment_info_id FROM
base.payment_info INNER JOIN
base.address ON billing_address_id = address_id WHERE
name_on_card = $1 AND
expiry = $2 AND
card_number = $3 AND
cvv = $4 AND
street_address = $5 AND
postal_code = $6 AND
province = $7;

-- Create a new address returning the ID of the newly created address
INSERT INTO base.address
(street_address, postal_code, province)
VALUES ($1, $2, $3)
RETURNING address_id;

-- Create a new payment info returning the ID of the newly created payment info
INSERT INTO base.payment_info
(name_on_card, expiry, card_number, cvv, billing_address_id)
VALUES ($1, $2, $3, $4)
RETURNING payment_info_id;

-- Remove everything from a cart
DELETE FROM base.in_cart WHERE customer_id = $1;

-- Add a book to an order
INSERT INTO base.in_order
VALUES ($1, $2, $3);

-- Remove stock from a book
UPDATE base.book SET stock = stock - $1 WHERE isbn = $2;

-- Create a new order returning the new ID
INSERT INTO base.orders
(customer_id, shipping_address_id, tracking_number, order_status, order_date, payment_info_id)
VALUES
($1, $2, $3, $4, $5, $6)
RETURNING order_id;

-- Get the books in an order
SELECT * FROM base.in_order INNER JOIN base.book ON base.in_order.isbn = base.book.isbn WHERE order_id = $1;

-- Get the complete information for an order
SELECT
order_id,
add.street_address,
add.postal_code,
add.province,
bill.street_address AS bill_street_address,
bill.postal_code AS bill_postal_code,
bill.province AS bill_province,
order_status,
order_date,
tracking_number,
name_on_card,
card_number,
expiry,
cvv
FROM
base.orders AS orders
INNER JOIN base.address AS add ON orders.shipping_address_id = add.address_id
INNER JOIN base.payment_info AS payment ON orders.payment_info_id = payment.payment_info_id
INNER JOIN base.address AS bill ON payment.billing_address_id = bill.address_id
WHERE order_id = $1;

-- Get the complete information for all of a customer's orders given a customer id
SELECT
order_id,
add.street_address,
add.postal_code,
add.province,
bill.street_address AS bill_street_address,
bill.postal_code AS bill_postal_code,
bill.province AS bill_province,
order_status,
order_date,
tracking_number,
name_on_card,
card_number,
expiry,
cvv
FROM
base.orders AS orders
INNER JOIN base.address AS add ON orders.shipping_address_id = add.address_id
INNER JOIN base.payment_info AS payment ON orders.payment_info_id = payment.payment_info_id
INNER JOIN base.address AS bill ON payment.billing_address_id = bill.address_id
WHERE customer_id = $1;

-- Discontinue a book based on ISBN
UPDATE base.book SET discontinued = true WHERE isbn = $1;

-- Undiscontinue a book based on ISBN
UPDATE base.book SET discontinued = false WHERE isbn = $1;

-- Create a new publisher
INSERT INTO base.publisher
            (company_name, phone_number, bank_number, address_id, email) VALUES ($1, $2, $3, $4, $5) RETURNING publisher_id;

-- Get all the sales
SELECT * FROM base.sales;

-- Get the sales by date of a certain publisher
SELECT order_date, sum(quantity) as quantity FROM base.raw_sales_data WHERE publisher_id = $1 GROUP BY (order_date);

-- Get the sales by date of a certain author
SELECT order_date, sum(quantity) as quantity FROM base.raw_sales_data WHERE author_name = $1 GROUP BY (order_date);

-- Get the sales by genre of a certain genre
SELECT order_date, sum(quantity) as quantity FROM base.raw_sales_data WHERE author_name = $1 GROUP BY (order_date);

-- Get the top authors by sales
SELECT author_name FROM base.raw_sales_data WHERE order_date > NOW() - interval '1 month' GROUP BY (author_name) ORDER BY sum(quantity) desc limit 10;

-- Get the top genres by sales
SELECT genre FROM base.raw_sales_data WHERE order_date > NOW() - interval '1 month' GROUP BY (genre) ORDER BY sum(quantity) desc limit 10;

-- Create a book
INSERT INTO base.book (isbn, author_name, genre, publisher_id, num_pages, price, author_royalties, reorder_threshold, title, stock, discontinued)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11);

-- Get all publishers
SELECT * FROM base.publisher;

-- Get all owners
SELECT owner_id, name, email FROM base.owner;

-- Get all customers
SELECT customer_id, name, email FROM base.customer;

-- Delete a customer account
DELETE FROM base.customer WHERE customer_id = $1 ON DELETE CASCADE;

-- Delete an owner account
DELETE FROM base.owner WHERE owner_id = $1 ON DELETE CASCADE;

-- Get all owner IDs
SELECT owner_id FROM base.owner;

-- Get all restock orders
SELECT * FROM base.restock_order;

-- Insert a new book collection and returns its ID
INSERT INTO base.book_collection (curator_owner_id, name) VALUES ($1, $2) RETURNING collection_id;

-- Add book to a curated collection
INSERT INTO base.in_collection (collection_id, isbn) VALUES ($1, $2);

-- Remove book from a curated collection
DELETE FROM base.in_collection WHERE isbn = $1 AND collection_id = $2;

-- Get book information in a curated collection
SELECT base.book_collection.name AS name,
curator_owner_id, base.owner.name AS curator_name
FROM base.book_collection INNER JOIN
base.owner ON base.book_collection.curator_owner_id = base.owner.owner_id
WHERE collection_id = $1;

-- Get information of books in a collection
SELECT * FROM base.in_collection INNER JOIN base.book USING (isbn) WHERE collection_id = $1;

-- Get the collections with the curator information
SELECT collection_id, base.book_collection.name AS name,
curator_owner_id, base.owner.name AS curator_name
FROM base.book_collection INNER JOIN
base.owner ON base.book_collection.curator_owner_id = base.owner.owner_id;

-- Get the collections with the curator information curated by a specific owner
SELECT collection_id, base.book_collection.name AS name,
curator_owner_id, base.owner.name AS curator_name
FROM base.book_collection INNER JOIN
base.owner ON base.book_collection.curator_owner_id = base.owner.owner_id
WHERE curator_owner_id = $1

-- See if owner is the curator of a collection
SELECT * FROM base.book_collection WHERE collection_id = $1 AND
curator_owner_id = $2;
