use std::collections::HashMap;

use crate::db::conn::DbConn;
use crate::db::error::{CartError, OrderError, StateError};
use crate::db::query::{
    add_to_cart, cart_set_book_quantity, get_books, get_books_for_order, get_customer_cart,
    get_customer_info, get_customer_orders_info, get_order_info, try_create_new_customer,
    validate_customer_login, Expiry,
};
use crate::request_guards::state::SessionType;
use crate::schema::entities::{Book, PostgresInt, ISBN};
use crate::schema::joined::Order;
use crate::schema::no_id::{Address, PaymentInfo};
use crate::schema::{self, no_id};
use chrono::{Duration, Local};
use rand::{RngCore, SeedableRng};
use rocket::form::validate::Contains;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::tera::Context;
use rocket_dyn_templates::Template;
use serde::Serialize;

use crate::{request_guards::*, SessionTokenState};

async fn render_error_template<T: AsRef<str>>(
    error: T,
    conn: &DbConn,
    customer: &Option<Customer>,
) -> Template {
    let mut context = Context::new();

    add_customer_info(conn, customer, &mut context).await;

    context.insert("error", error.as_ref());

    Template::render("error", context.into_json())
}

async fn add_customer_info(conn: &DbConn, customer: &Option<Customer>, context: &mut Context) {
    if let Some(customer) = customer {
        if let Ok(customer_info) = get_customer_info(&conn, customer.customer_id).await {
            context.insert("customer", &customer_info);
        } else {
            context.insert("customer", &crate::schema::joined::Customer::default());
        }

        if let Ok(cart) = get_customer_cart(&conn, customer.customer_id).await {
            context.insert("cart_size", &cart.len());
        } else {
            context.insert("cart_size", &0);
        }
    }
}

#[get("/")]
pub async fn index(conn: DbConn, customer: Option<Customer>) -> Template {
    let mut context = Context::new();
    add_customer_info(&conn, &customer, &mut context).await;

    let books = get_books(&conn).await;
    if let Ok(books) = books {
        context.insert("books", &books);

        if let Some(customer) = customer {
            let customer_info = get_customer_info(&conn, customer.customer_id).await;

            if let Ok(customer_info) = customer_info {
                context.insert("customer", &customer_info);
            }
        }

        Template::render("index", context.into_json())
    } else {
        render_error_template(
            format!("Could not query books: {:?}", books),
            &conn,
            &customer,
        )
        .await
    }
}

#[get("/login")]
pub async fn login_page() -> Template {
    let context = HashMap::<&str, &str>::new();
    Template::render("login", &context)
}

#[get("/login/failed/<e>")]
pub async fn login_failed(e: &str, conn: DbConn) -> Template {
    render_error_template(format!("Login failed: {}", e), &conn, &None).await
}

#[get("/register")]
pub async fn register_page() -> Template {
    let context = HashMap::<&str, &str>::new();
    Template::render("registration", &context)
}

#[get("/register/failed/<reason>")]
pub async fn register_failed(reason: &str, conn: DbConn) -> Template {
    render_error_template(format!("Registration failed: {}", reason), &conn, &None).await
}

#[get("/customer")]
pub async fn customer_page(cust: Customer, conn: DbConn) -> Template {
    let mut context = Context::new();

    match get_customer_info(&conn, cust.customer_id.clone()).await {
        Ok(v) => match v {
            Some(v) => v,
            None => {
                return render_error_template("No such customer", &conn, &None).await;
            }
        },
        Err(e) => {
            return render_error_template(format!("Server error: {}", e), &conn, &None).await;
        }
    };

    add_customer_info(&conn, &Some(cust), &mut context).await;
    Template::render("customer", context.into_json())
}

#[get("/owner")]
pub async fn owner_page() -> Template {
    let context = HashMap::<&str, &str>::new();
    Template::render("owner", &context)
}

#[derive(FromForm)]
pub struct Login<'r> {
    email: &'r str,
    password: &'r str,
}

#[post("/login", data = "<login_data>")]
pub async fn login(
    conn: DbConn,
    login_data: Form<Login<'_>>,
    session_tokens: &State<SessionTokenState>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    match validate_customer_login(&conn, login_data.email, login_data.password).await {
        Ok(customer_id) => {
            let mut rng = rand_chacha::ChaCha12Rng::from_entropy();
            let mut token: [u8; 32] = [0; 32];
            rng.fill_bytes(&mut token);

            let token = base64::encode(token);

            cookies.add_private(Cookie::new(CUST_SESSION_COOKIE_NAME, token.clone()));

            let mut session_tokens = session_tokens.lock().await;

            let expiry = Local::now() + Duration::days(30);

            session_tokens.insert(token, (SessionType::Customer(customer_id), expiry));
            Redirect::to(uri!(customer_page()))
        }
        Err(e) => match e {
            _ => Redirect::to(uri!(login_failed("Invalid email/password"))),
        },
    }
}

#[derive(FromForm)]
pub struct Register<'r> {
    email: &'r str,
    name: &'r str,
    password: &'r str,
    street_address: &'r str,
    postal_code: &'r str,
    province: &'r str,
    name_on_card: &'r str,
    card_number: &'r str,
    expiry: &'r str,
    cvv: &'r str,
    billing_street_address: &'r str,
    billing_postal_code: &'r str,
    billing_province: &'r str,
}

#[post("/register", data = "<register_data>")]
pub async fn register(conn: DbConn, register_data: Form<Register<'_>>) -> Redirect {
    let Register {
        email,
        name,
        password,
        street_address,
        postal_code,
        province,
        name_on_card,
        card_number,
        expiry,
        cvv,
        billing_street_address,
        billing_postal_code,
        billing_province,
    } = *register_data;

    let address = no_id::Address::new(street_address, postal_code, province);
    let billing_address = no_id::Address::new(
        billing_street_address,
        billing_postal_code,
        billing_province,
    );

    let expiry = if let Some(e) = Expiry::from_str(expiry) {
        e
    } else {
        return Redirect::to(uri!(register_failed(format!(
            "{:?}",
            "Invalid Credit Card Expiry"
        ))));
    };

    let payment_info =
        no_id::PaymentInfo::new(name_on_card, expiry, card_number, cvv, billing_address);

    match try_create_new_customer(&conn, email, password, name, address, payment_info).await {
        Ok(_) => Redirect::to(uri!(customer_page())),
        Err(e) => Redirect::to(uri!(register_failed(format!("{:?}", e)))),
    }
}

#[get("/book/<isbn>")]
pub async fn book(conn: DbConn, isbn: &str, customer: Option<Customer>) -> Template {
    let mut context = Context::new();
    add_customer_info(&conn, &customer, &mut context).await;

    match isbn.parse::<i32>() {
        Ok(isbn) => {
            let books = get_books(&conn).await;

            match books {
                Ok(books) => match books.iter().find(|book| book.isbn == isbn) {
                    Some(book) => {
                        context.insert("book", &book);
                        Template::render("book", context.into_json())
                    }
                    None => {
                        render_error_template(
                            format!("No book with ISBN: {}", isbn),
                            &conn,
                            &customer,
                        )
                        .await
                    }
                },
                Err(e) => {
                    render_error_template(format!("Server error: {}", e), &conn, &customer).await
                }
            }
        }
        Err(_) => {
            render_error_template(format!("{} is not a valid ISBN", isbn), &conn, &customer).await
        }
    }
}

#[get("/customer/cart")]
pub async fn customer_cart_page(conn: DbConn, customer: Option<Customer>) -> Template {
    let mut context = Context::new();
    add_customer_info(&conn, &customer, &mut context).await;

    use schema::entities::Book;

    match customer {
        Some(customer) => match get_customer_cart(&conn, customer.customer_id).await {
            Ok(cart) => match get_books(&conn).await {
                Ok(books) => {
                    context.insert("cart", &cart);

                    let isbns: Vec<ISBN> = cart.iter().map(|c| c.0).collect();
                    let quantities: HashMap<ISBN, u32> = cart.iter().map(|p| p.clone()).collect();

                    #[derive(serde::Serialize)]
                    struct BookWithQuantity {
                        book: Book,
                        quantity: u32,
                    }

                    let books: Vec<BookWithQuantity> = books
                        .into_iter()
                        .filter(|b| isbns.contains(&b.isbn))
                        .map(|b| BookWithQuantity {
                            book: b.clone(),
                            quantity: *quantities.get(&b.isbn).unwrap(),
                        })
                        .collect();

                    context.insert("books", &books);
                    Template::render("customer_cart", context.into_json())
                }
                Err(e) => {
                    render_error_template(
                        format!("Error fetching books: {}", e),
                        &conn,
                        &Some(customer),
                    )
                    .await
                }
            },
            Err(_) => {
                render_error_template(
                    format!("Could not fetch cart for {}", customer.customer_id),
                    &conn,
                    &Some(customer),
                )
                .await
            }
        },
        None => render_error_template("Please login to see your cart.", &conn, &customer).await,
    }
}

#[put("/customer/cart/add/<isbn>")]
pub async fn customer_cart_add(conn: DbConn, customer: Customer, isbn: ISBN) -> Status {
    match add_to_cart(&conn, customer.customer_id, isbn).await {
        Ok(_) => Status::Ok,
        Err(_) => Status::InternalServerError,
    }
}

#[put("/customer/cart/quantity/<isbn>/<quantity>")]
pub async fn customer_cart_set_quantity(
    conn: DbConn,
    customer: Customer,
    isbn: ISBN,
    quantity: u32,
) -> Result<(), rocket::response::status::Custom<String>> {
    use rocket::response::status;
    cart_set_book_quantity(&conn, customer.customer_id, isbn, quantity)
        .await
        .map_err(|e| match e {
            CartError::NotEnoughStock => {
                status::Custom(Status::Conflict, "Insufficient book stock".to_owned())
            }
            CartError::DBError(_) => status::Custom(Status::InternalServerError, e.to_string()),
        })
}

#[post("/account/logout")]
pub async fn account_logout(
    cookies: &CookieJar<'_>,
    session_tokens: &State<SessionTokenState>,
) -> () {
    let mut session_tokens = session_tokens.lock().await;
    if let Some(cookie) = cookies.get_private(CUST_SESSION_COOKIE_NAME) {
        session_tokens.remove(cookie.value());
    }
}

#[get("/checkout")]
pub async fn checkout_page(conn: DbConn, customer: Customer) -> Template {
    let mut context = Context::new();
    let customer_id = customer.customer_id;

    use schema::entities::Book;

    add_customer_info(&conn, &Some(customer), &mut context).await;
    match get_customer_cart(&conn, customer_id).await {
        Ok(cart) => match get_books(&conn).await {
            Ok(books) => {
                context.insert("cart", &cart);

                let isbns: Vec<ISBN> = cart.iter().map(|c| c.0).collect();
                let quantities: HashMap<ISBN, u32> = cart.iter().map(|p| p.clone()).collect();

                #[derive(serde::Serialize)]
                struct BookWithQuantity {
                    book: Book,
                    quantity: u32,
                }

                let books: Vec<BookWithQuantity> = books
                    .into_iter()
                    .filter(|b| isbns.contains(&b.isbn))
                    .map(|b| BookWithQuantity {
                        book: b.clone(),
                        quantity: *quantities.get(&b.isbn).unwrap(),
                    })
                    .collect();

                context.insert("books", &books);

                Template::render("checkout_page", context.into_json())
            }
            Err(e) => {
                render_error_template(
                    format!("Error fetching books: {}", e),
                    &conn,
                    &Some(customer),
                )
                .await
            }
        },
        Err(e) => {
            render_error_template(format!("Server error: {}", e), &conn, &Some(customer)).await
        }
    }
}

#[derive(FromForm)]
pub struct CreateOrder<'r> {
    default_shipping: bool,
    default_payment: bool,
    street_address: &'r str,
    postal_code: &'r str,
    province: &'r str,
    name_on_card: &'r str,
    card_number: &'r str,
    expiry: &'r str,
    cvv: &'r str,
    billing_street_address: &'r str,
    billing_postal_code: &'r str,
    billing_province: &'r str,
}

#[post("/order/create", data = "<create_order>")]
pub async fn create_order_req(
    conn: DbConn,
    create_order: Form<CreateOrder<'_>>,
    customer: Customer,
) -> Template {
    let result: Result<PostgresInt, OrderError> = try {
        let cart = get_customer_cart(&conn, customer.customer_id).await?;

        let address = if create_order.default_shipping {
            None
        } else {
            Some(Address::new(
                create_order.street_address,
                create_order.postal_code,
                create_order.province,
            ))
        };

        let payment_info = if create_order.default_payment {
            None
        } else {
            let address = Address::new(
                create_order.billing_street_address,
                create_order.billing_postal_code,
                create_order.billing_province,
            );
            Some(PaymentInfo::new(
                create_order.name_on_card,
                Expiry::from_str(create_order.expiry).ok_or(StateError::new("Invalid expiry"))?,
                create_order.card_number,
                create_order.cvv,
                address,
            ))
        };

        crate::db::query::create_order(&conn, customer.customer_id, cart, address, payment_info)
            .await?
    };

    match result {
        Ok(order_id) => {
            let mut context = Context::new();
            context.insert("order_id", &order_id);
            Template::render("order_success", context.into_json())
        }
        Err(e) => {
            render_error_template(format!("Order error: {}", e), &conn, &Some(customer)).await
        }
    }
}

#[derive(Serialize, Debug)]
struct CensoredPaymentInfo {
    pub name_on_card: String,
    pub expiry: Expiry,
    pub censored_card_number: String,
    pub billing_address: Address,
}

#[derive(Serialize, Debug)]
struct BookWithQuantity {
    book: Book,
    quantity: u32,
}

#[derive(Serialize, Debug)]
struct CensoredOrder {
    pub order_id: PostgresInt,
    pub shipping_address: Address,
    pub tracking_number: String,
    pub order_status: String,
    pub order_date: String,
    pub payment_info: CensoredPaymentInfo,
    pub books: Vec<BookWithQuantity>,
}

fn censor_order(order: Order) -> CensoredOrder {
    let Order {
        order_id,
        shipping_address,
        tracking_number,
        order_status,
        order_date,
        payment_info,
        books,
    } = order;
    let PaymentInfo {
        name_on_card,
        expiry,
        card_number,
        cvv: _,
        billing_address,
    } = payment_info;

    let num_last_digits = usize::min(card_number.len(), 4);
    let censored_card_number = "*".repeat(12) + &card_number[card_number.len() - num_last_digits..];
    let censored_payment_info = CensoredPaymentInfo {
        name_on_card,
        expiry,
        censored_card_number,
        billing_address,
    };

    let books = books
        .into_iter()
        .map(|(book, quantity)| BookWithQuantity { book, quantity })
        .collect();

    CensoredOrder {
        order_id,
        shipping_address,
        tracking_number,
        order_status,
        order_date,
        payment_info: censored_payment_info,
        books,
    }
}

fn add_orders_to_context(orders: Vec<Order>, context: &mut Context) {
    let orders: Vec<CensoredOrder> = orders.into_iter().map(censor_order).collect();

    context.insert("orders", &orders);
}

#[get("/order/view")]
pub async fn orders_page(conn: DbConn, customer: Customer) -> Template {
    let mut context = Context::new();

    let customer_id = customer.customer_id;

    add_customer_info(&conn, &Some(customer), &mut context).await;

    match get_customer_orders_info(&conn, customer_id).await {
        Ok(orders) => {
            context.insert("num_orders", &orders.len());
            add_orders_to_context(orders, &mut context);
            Template::render("orders", context.into_json())
        }
        Err(e) => {
            render_error_template(format!("Server error: {}", e), &conn, &Some(customer)).await
        }
    }
}

#[get("/order/view/<order_id>")]
pub async fn view_order(conn: DbConn, customer: Customer, order_id: i32) -> Template {
    let mut context = Context::new();

    add_customer_info(&conn, &Some(customer), &mut context).await;

    match get_order_info(&conn, order_id).await {
        Ok(order_info) => match get_books_for_order(&conn, order_info).await {
            Ok(order) => {
                let censored_order = censor_order(order);
                context.insert("order", &censored_order);
                Template::render("order", context.into_json())
            }
            Err(e) => {
                render_error_template(format!("Server error: {}", e), &conn, &Some(customer)).await
            }
        },
        Err(e) => {
            render_error_template(format!("Server error: {}", e), &conn, &Some(customer)).await
        }
    }
}
