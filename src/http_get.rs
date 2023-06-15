// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

use curl::easy::Easy;

pub struct HttpGetter {
    curl: Easy,
}

impl HttpGetter {
    /// Creates a new [`HttpGetter`] for the given url.
    ///
    /// [`HttpGetter`]: HttpGetter
    pub fn new(url: &str) -> Result<Self, curl::Error> {
        let mut curl = Easy::new();

        curl.url(url)?;
        curl.write_function(|data| Ok(data.len()))?;

        Ok(Self { curl })
    }

    /// Performs a GET and returns the result.
    pub fn run(&mut self) -> Result<u32, curl::Error> {
        self.curl.perform()?;
        self.curl.response_code()
    }
}
