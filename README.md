# LookInnaBook

This is a small CRUD app simulating a book storefront written for the final project of Carleton's COMP 3005 course. It supports:

- Purchasing and shipping books
- Searching books by ISBN, title, author, etc.
- Customer accounts with default billing/shipping details
- Browsing past orders as a customer

- Curated book collections by owners
- Sales reports for owners
- Automatic restocking at custom thresholds
- Book management, creation, discontinuation, recontinuation

# Setup

Building the application is as simple as running

``` sh
cargo build
```

on nightly rust if you have the rust toolchain installed, if not, you can install it [here](https://rustup.rs/).

Other than that, the application expects a postgres url configuration in Rocket.toml. There is a schema dump in `schema_dump.sql` and the trigger functions are stored separately in `trigger.sql`.
