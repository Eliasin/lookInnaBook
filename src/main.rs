#![feature(try_blocks)]
#[macro_use]
extern crate rocket;

mod db;
mod endpoints;
mod request_guards;
mod schema;

use std::sync::Arc;

use db::conn::DbConn;
use endpoints::*;
use rocket::{fs::FileServer, futures::lock::Mutex};
use rocket_dyn_templates::Template;

use request_guards::state::SessionTokens;

pub type SessionTokenState = Arc<Mutex<SessionTokens>>;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                book,
                index,
                login,
                login_page,
                login_failed,
                customer_page,
                register,
                register_page,
                register_failed,
                customer_cart_page,
                customer_cart_add,
                customer_cart_set_quantity,
                account_logout,
                checkout_page,
                create_order_req,
                orders_page,
                view_order,
                owner_login_page,
                owner_login,
                book_management,
                discontinue_books_endpoint,
                undiscontinue_books_endpoint,
                create_publisher_page,
                create_publisher,
                sales_report_image_publishers,
                sales_report_image_authors,
                sales_report_image_genre,
                reports_page,
                create_book_page,
                create_book_endpoint,
                manage_accounts,
                delete_owner_page,
                delete_customer_page,
                delete_customer_endpoint,
                delete_owner_endpoint,
                delete_success_page,
                error_page,
                create_owner,
                restock_order_page,
                manage_collections_page,
                add_collection_endpoint,
                add_books_to_collection_endpoint,
                remove_books_from_collection_endpoint,
                view_collection,
                collections_page,
            ],
        )
        .mount("/style", FileServer::from("style/"))
        .manage(SessionTokenState::new(Mutex::new(SessionTokens::new())))
        .attach(DbConn::fairing())
        .attach(Template::fairing())
}
