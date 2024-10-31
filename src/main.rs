//! ## Task Description

//! ## Task Description
//!
//! The goal is to develop a backend service for shortening URLs using CQRS
//! (Command Query Responsibility Segregation) and ES (Event Sourcing)
//! approaches. The service should support the following features:
//!
//! ## Functional Requirements
//!
//! ### Creating a short link with a random slug
//!
//! The user sends a long URL, and the service returns a shortened URL with a
//! random slug.
//!
//! ### Creating a short link with a predefined slug
//!
//! The user sends a long URL along with a predefined slug, and the service
//! checks if the slug is unique. If it is unique, the service creates the short
//! link.
//!
//! ### Counting the number of redirects for the link
//!
//! - Every time a user accesses the short link, the click count should
//!   increment.
//! - The click count can be retrieved via an API.
//!
//! ### CQRS+ES Architecture
//!
//! CQRS: Commands (creating links, updating click count) are separated from
//! queries (retrieving link information).
//!
//! Event Sourcing: All state changes (link creation, click count update) must be
//! recorded as events, which can be replayed to reconstruct the system's state.
//!
//! ### Technical Requirements
//!
//! - The service must be built using CQRS and Event Sourcing approaches.
//! - The service must be possible to run in Rust Playground (so no database like
//!   Postgres is allowed)
//! - Public API already written for this task must not be changed (any change to
//!   the public API items must be considered as breaking change).

#![allow(unused_variables, dead_code)]

use std::collections::HashMap;
use rand::prelude::IndexedRandom;
use commands::CommandHandler;
use queries::QueryHandler;

/// All possible errors of the [`UrlShortenerService`].
#[derive(Debug, PartialEq)]
pub enum ShortenerError {
    /// This error occurs when an invalid [`Url`] is provided for shortening.
    InvalidUrl,

    /// This error occurs when an attempt is made to use a slug (custom alias)
    /// that already exists.
    SlugAlreadyInUse,

    /// This error occurs when the provided [`Slug`] does not map to any existing
    /// short link.
    SlugNotFound,
}

/// A unique string (or alias) that represents the shortened version of the
/// URL.
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Slug(pub String);

impl Eq for Slug {}

/// The original URL that the short link points to.
#[derive(Clone, Debug, PartialEq)]
pub struct Url(pub String);

/// Shortened URL representation.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortLink {
    /// A unique string (or alias) that represents the shortened version of the
    /// URL.
    pub slug: Slug,

    /// The original URL that the short link points to.
    pub url: Url,
}

/// Statistics of the [`ShortLink`].
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// [`ShortLink`] to which this [`Stats`] are related.
    pub link: ShortLink,

    /// Count of redirects of the [`ShortLink`].
    pub redirects: u64,
}

/// Commands for CQRS.
pub mod commands {
    use super::{ShortLink, ShortenerError, Slug, Url};

    /// Trait for command handlers.
    pub trait CommandHandler {
        /// Creates a new short link. It accepts the original url and an
        /// optional [`Slug`]. If a [`Slug`] is not provided, the service will generate
        /// one. Returns the newly created [`ShortLink`].
        ///
        /// ## Errors
        ///
        /// See [`ShortenerError`].
        fn handle_create_short_link(
            &mut self,
            url: Url,
            slug: Option<Slug>,
        ) -> Result<ShortLink, ShortenerError>;

        /// Processes a redirection by [`Slug`], returning the associated
        /// [`ShortLink`] or a [`ShortenerError`].
        fn handle_redirect(
            &mut self,
            slug: Slug,
        ) -> Result<ShortLink, ShortenerError>;
        
        /// Updates the [Url] of a [ShortLink] using a given [Slug].
        fn handle_change_short_link(
            &mut self,
            slug: Slug,
            new_url: Url
        ) -> Result<ShortLink, ShortenerError>;
    }
}

/// Queries for CQRS
pub mod queries {
    use super::{ShortenerError, Slug, Stats};

    /// Trait for query handlers.
    pub trait QueryHandler {
        /// Returns the [`Stats`] for a specific [`ShortLink`], such as the
        /// number of redirects (clicks).
        ///
        /// [`ShortLink`]: super::ShortLink
        fn get_stats(&self, slug: Slug) -> Result<Stats, ShortenerError>;
    }
}

/// CQRS and Event Sourcing-based service implementation
pub struct UrlShortenerService {
    map: HashMap<Slug, ShortLink>,
    stats: HashMap<Slug, Stats>,
    slug_alphabet: Vec<char>,
}

impl UrlShortenerService {
    /// Creates a new instance of the service
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            stats: HashMap::new(),
            slug_alphabet: "aAbBcCdDeEfFgGhHjJkKmMnNpPqQrRsStTuUvVwWxXyYzZ0123456789".chars().collect(),
        }
    }

    fn generate_unique_slug(&self) -> Slug {
        let mut rng = rand::thread_rng();

        loop {
            let slug = Slug(String::from_iter(
                self.slug_alphabet.choose_multiple(&mut rng, 6)
            ));

            if !self.map.contains_key(&slug) {
                return slug;
            }
        }
    }
}

impl commands::CommandHandler for UrlShortenerService {

    fn handle_create_short_link(
        &mut self,
        url: Url,
        slug: Option<Slug>,
    ) -> Result<ShortLink, ShortenerError> {

        if !validate_url(&url) {
            return Err(ShortenerError::InvalidUrl);
        };

        // Generate new slug if not provided.
        let slug = slug.unwrap_or_else(|| self.generate_unique_slug());

        // Slug must be unique.
        if self.map.contains_key(&slug) {
            return Err(ShortenerError::SlugAlreadyInUse);
        };

        // We create stats entry with 0 redirects here to avoid panics in
        // handle_redirect() and get_stats().
        let link = ShortLink { slug: slug.clone(), url };
        self.map.insert(slug.clone(), link.clone());
        self.stats.insert(slug.clone(), Stats { link: link.clone(), redirects: 0 });

        Ok(link)
    }

    fn handle_redirect(
        &mut self,
        slug: Slug,
    ) -> Result<ShortLink, ShortenerError> {
        
        if let Some(link) = self.map.get(&slug) {
            self.stats.get_mut(&slug).unwrap()
                .redirects += 1;

            Ok(link.clone())
        } else {
            Err(ShortenerError::SlugNotFound)
        }
    }
    
    /// Updates the [Url] of a [ShortLink] using a given [Slug].
    fn handle_change_short_link(
        &mut self,
        slug: Slug,
        new_url: Url
    ) -> Result<ShortLink, ShortenerError> {

        if !validate_url(&new_url) {
            return Err(ShortenerError::InvalidUrl);
        };

        match self.map.get_mut(&slug) {
            Some(link) => {
                link.url = new_url;
                Ok(link.clone())
            }
            None => Err(ShortenerError::SlugNotFound),
        }
    }
}

fn validate_url(url: &Url) -> bool {
    if !url.0.starts_with("http://") && !url.0.starts_with("https://") {
        return false;
    }

    // Проверяем наличие домена после протокола
    let domain_part = url.0.split("://").nth(1);
    match domain_part {
        Some(domain) => !domain.is_empty(),
        None => false
    }
}

impl queries::QueryHandler for UrlShortenerService {
    fn get_stats(
        &self,
        slug: Slug
    ) -> Result<Stats, ShortenerError> {

        // Slug must be in both map and stats collection. If only in first then we panic.
        if self.map.contains_key(&slug) {
            return Ok(self.stats.get(&slug).unwrap().clone());
        }

        Err(ShortenerError::SlugNotFound)
    }
}

fn main() {
    let mut svc = UrlShortenerService::new();

    let url = Url("https://docs.rs".to_string());    
    let link = match svc.handle_create_short_link(url.clone(), None) {
        Err(e) => {
            println!("Can not create short link. {:?}", e);
            return;
        },
        Ok(s) => s
    };

    println!("Created short link. Slug: {}", link.slug.0);

    let redir_link = svc.handle_redirect(link.slug.clone())
        .expect("Redirect error");

    assert_eq!(link, redir_link);
    println!("Slug redirects to link: {}", redir_link.url.0);

    match svc.handle_create_short_link(url.clone(), Some(link.slug.clone())) {
        Err(e) => println!("Can not create new short link with Slug {}. {:?}", link.slug.0, e),
        Ok(link) => panic!("Same Slug was created!")
    };

    let new_url = Url("https://docs.rs/tokio/latest/tokio/".to_string());
    let new_short = match svc.handle_change_short_link(link.slug.clone(), new_url) {
        Err(e) => {
            println!("Can not change url to link. {:?}", e);
            return;
        },
        Ok(s) => s
    };

    let redir_link2 = svc.handle_redirect(link.slug.clone())
        .expect("Redirect error");

    assert_ne!(redir_link, redir_link2);
    println!("Slug redirects to link: {} after change URL", redir_link2.url.0);

    let stats = svc.get_stats(link.slug).unwrap();
    println!("Redirect count for {} is: {}", stats.link.slug.0, stats.redirects);

    let non_exists = svc.generate_unique_slug();
    match svc.get_stats(non_exists.clone()) {
        Err(e) => println!("Error getting stats for non-existing Slug {}: {:?}", non_exists.0, e),
        Ok(s) => panic!()
    }

    assert_eq!(stats.redirects, 2);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_url() {
        assert_eq!(true, validate_url(&Url("http://ya.ru".to_string())));
        assert_eq!(false, validate_url(&Url("abc".to_string())));
    }
}