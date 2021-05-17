use std::error::Error;
use std::collections::HashMap;
use std::sync::Once;
use std::cell::RefCell;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use reqwest::blocking::ClientBuilder;
use reqwest::header::HeaderValue;
use reqwest::header::HeaderMap;
use reqwest::cookie::Cookie;
use reqwest::redirect::Policy;
use scraper::Html;
use scraper::Selector;
use scraper::element_ref::Select;
use scraper::ElementRef;
use env_logger;
use log;
use log::LevelFilter;
use crate::html::init_logger;
use crate::error::HNError;
use crate::html::parse::extract_listings;
use crate::html::parse::extract_comments;
use crate::html::parse::extract_fnid;
use crate::html::parse::create_comment_tree;
use crate::html::models::Score;
use crate::html::models::Id;
use crate::html::models::Listing;
use crate::html::models::Date;
use crate::html::models::Comment;

/* Note:
 *
 * Consider how fields like `score` and `id` should be shared between
 * firebase and html clients
 *
 * Selector::parse() returns an Error type which doesn't implement
 * std::error::Error, and therefore doesn't work using Result<T, Box<dyn Error>>
 * as a return type. Maybe open a pull request on the lib to implement
 * std::error::Error.
 *
 * Because all the methods are implemented in a trait, there is no concept
 * of public vs. private. Maybe consider not using the trait? Or maybe
 * consider not having seperate structs for auth and unauth
 * */


/* Todo:
 *
 * Add a `comment` attribute to the Listing struct, and have it be the comment
 * count displayed on a Listing.
 *
 * If Item and Struct are basically all the same information, consider representing
 * them as a single struct.
 *
 * Consider placing all HTML parsing logic into the `extract` function
 * */

const URL_LOGIN: &'static str =         "https://news.ycombinator.com/login";
const URL_SUBMIT_FORM: &'static str =   "https://news.ycombinator.com/submit";
const URL_SUBMIT: &'static str =        "https://news.ycombinator.com/r";

lazy_static! {
    static ref FNID_REGEX: Regex =  Regex::new(r#"<input.*value="(.+?)".*>"#).unwrap();
}

// pub type Score=u32;
// pub type Id=u32;

// pub struct Date(pub u16, pub u8, pub u8);

// #[derive(Debug)]
// pub struct Listing {
//     pub title: String,
//     pub id: Id,
//     pub score: Option<Score>,
//     pub user: Option<String>,
//     // comments: u32,
//     pub url: String,
// }

pub struct Client {
    http_client: reqwest::blocking::Client,
    username: String,
    password: String,
    cookie: RefCell<Option<(String, String)>>,
}

impl Client {

    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: String::from(username),
            password: String::from(password),
            http_client: reqwest::blocking::Client::new(),
            cookie: RefCell::new(None),
        }
    }

    fn fail_unauthenticated(&self) -> Box<HNError> {
        HNError::boxed("Cannot perform action because client is unauthenticated")
    }

    fn cookie(&self) -> Result<String, Box<dyn Error>> {
        match *self.cookie.borrow() {
            None => Err(self.fail_unauthenticated()),
            Some(ref cookie) => Ok(format!("{}={};", cookie.0, cookie.1))
        }
    }

    fn submit(
        &self,
        title: String,
        url: Option<String>,
        text: Option<String>,
    ) -> Result<(), Box<dyn Error>> {

        if self.cookie.borrow().is_none() {
            return Err(self.fail_unauthenticated());
        }

        let mut formdata = HashMap::new();
        formdata.insert("fnid", self.get_fnid()?);
        formdata.insert("fnop", "submit-page".to_string());
        formdata.insert("url", url.unwrap_or("".to_string()));
        formdata.insert("text", text.unwrap_or("".to_string()));
        log::debug!("submit post body = {:?}", formdata);
        formdata.insert("title", title);
        
        let cookie: HeaderValue = self.cookie()?.parse().unwrap();
        let req = self.http_client.post(URL_SUBMIT)
            .header("Cookie", cookie)
            .form(&formdata);
        log::debug!("submit post request = {:?}", req);
        let resp = req.send()?;
        log::debug!("submit post response = {:?}", resp);
        
        Ok(())

    }
    
    fn get_fnid(&self) -> Result<String, Box<dyn Error>> {
        if self.cookie.borrow().is_none() {
            return Err(self.fail_unauthenticated());
        }
    
        let cookie: HeaderValue = self.cookie()?.parse().unwrap();
        let req = self.http_client
            .get(URL_SUBMIT_FORM)
            .header("Cookie", cookie);
        log::debug!("submit form request = {:?}", req);
        let resp = req.send()?;
        log::debug!("submit form response = {:?}", resp);
        let body = resp.text()?;
        let dom = Html::parse_document(&body);
        
        // Underlying library doesn't implement std::error::Error on their
        // Error structs, so I can't include it as the src error in my struct
        let selector = match Selector::parse("input[name='fnid']") {
            Err(_src) => {
                return Err(HNError::boxed("Unable to parse css query selector."));
            },
            Ok(selector) => selector,
        };
    
        let result: Vec<ElementRef> = dom.select(&selector).collect();
        let el = match result.get(0) {
            Some(el) => el,
            None => {
                return Err(HNError::boxed("Could not locate fnid input from submission form."));
            }
        };
        let fnid = extract_fnid(el)?;
    
        Ok(fnid)
    }

    pub fn login(&self) -> Result<(), Box<dyn Error>> {
        
        // Create form-data body parameters
        let mut formdata = HashMap::new();
        formdata.insert("acct", &self.username);
        formdata.insert("pw", &self.password);
        let goto = "newest".to_string();
        formdata.insert("goto", &goto);

        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "hnews client/0.0.1".parse().unwrap());

        // Login request requires no redirect on response, therefore we build a 
        // new one rather than referencing self.http_client.
        // TODO: Is there a better way to accomodate this?
        let client = ClientBuilder::new()
            .redirect(Policy::none())
            .build()?;

        // Send login request
        let req = client.post(URL_LOGIN)
            .headers(headers)
            .form(&formdata);
        log::debug!("login req = {:?}", req);
        let resp = req.send()?;
        log::debug!("login resp = {:?}", resp);

        // Store user session cookie
        let cookies: Vec<Cookie> = resp.cookies().collect();
        let cookie = cookies.get(0)
            .ok_or("Unable to retrieve user cookie")?;
        let cookie = Some((cookie.name().to_string(), cookie.value().to_string()));

        // Store on client instance field
        *self.cookie.borrow_mut() = cookie;
        println!("cookie = {:?}", self.cookie);

        Ok(())
    }
    
    pub fn item(&self, id: Id) -> Result<Listing, Box<dyn Error>> {
        // Retrieve front page HTML
        let url = format!("https://news.ycombinator.com/item?id={}", id);
        let req = self.http_client.get(&url);
        let resp = req.send()?;
        let text = resp.text()?;
        let html = Html::parse_document(&text);

        // Note: There is an assumption here that given an item ID, we should
        // only extract one listing from a page. Therefore, we can simply pop once
        // from the Vec obtained by extract listings.

        let item = extract_listings(&html)?
            .pop()
            .ok_or(format!("Did not find item {}", id))?;

        Ok(item)
    }

    // TODO: I don't love this with respect to API ergonomics. Refactor this functionality
    // in to something more well connected to the actual data model (i.e. posts,
    // comments, users, etc.)
    pub fn _comments(&self, id: Id) -> Result<Vec<Comment>, Box<dyn Error>> {
        let url = format!("https://news.ycombinator.com/item?id={}", id);
        let req = self.http_client.get(&url);
        let resp = req.send()?;
        let text = resp.text()?;
        let html = Html::parse_document(&text);
        let comments = extract_comments(&html)?;
        let comment_tree = create_comment_tree(comments);
        
        Ok(comment_tree)
    }

    pub fn news(&self) -> Result<Vec<Listing>, Box<dyn Error>> {
        self.listings("https://news.ycombinator.com/news")
    }

    pub fn past(&self, date: Date) -> Result<Vec<Listing>, Box<dyn Error>> {
        let url = format!("https://news.ycombinator.com/front?day={}-{}-{}",
            date.0, date.1, date.2);

        self.listings(&url)
    }

    /// Retrieve a page of HackerNews Listings, such as that delivered from:
    /// * `https://news.ycombinator.com/`
    /// * `https://news.ycombinator.com/newest`
    /// * `https://news.ycombinator.com/front`
    /// * `https://news.ycombinator.com/newcomments`
    /// * `https://news.ycombinator.com/ask`
    /// * `https://news.ycombinator.com/show`
    /// * `https://news.ycombinator.com/jobs`
    pub fn listings(&self, url: &str) -> Result<Vec<Listing>, Box<dyn Error>> {
        let req = self.http_client.get(url);
        let resp = req.send()?;
        let text = resp.text()?;
        let html = Html::parse_document(&text);
        let listings = extract_listings(&html)?;

        Ok(listings)
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    static LOG_INIT: Once = Once::new(); 
    
    fn setup() {
        LOG_INIT.call_once(|| {
            init_logger()
        });
    }

    #[test]
    fn test_news() -> Result<(), Box<dyn Error>> {
        setup();
        let client = Client::new("filler_user", "filler_pwd");
        let listings = client.news()?;
        log::debug!("test_news listings = {:#?}", listings);

        Ok(())
    }

    #[test]
    fn test_item() -> Result<(), Box<dyn Error>> {
        setup();
        let client = Client::new("filler_user", "filler_pwd");
        let item = client.item(25925926)?;
        log::debug!("test_item item = {:#?}", item);

        Ok(())
    }

    #[test]
    fn test_login() -> Result<(), Box<dyn Error>> {
        let user: String = std::env::var("HN_USER")?;
        let pwd: String = std::env::var("HN_PASS")?;
        println!("user = {:?}", user);
        println!("pwd = {:?}", pwd);
        let mut client = Client::new(&user, &pwd);
        client.login()?;

        Ok(())
    }

}