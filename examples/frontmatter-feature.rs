//! Example to show how to use frontmatter feature.
//!
//! This example turns all frontmatter values to uppercase
//! and formats a "date" value.
//!
//! Another use case for frontmatter is modifying the book's theme
//! to place frontmatter variables in HTML.
use crate::all_caps_lib::FrontmatterPreprocessor;
use mdbook::book::Book;
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use semver::{Version, VersionReq};
use std::io;
use std::process;

/// Main function for preprocessing data in frontmatter
fn main() {
    // lightweight approach to  capture args from env
    let args: Vec<String> = std::env::args().collect();
    let preprocessor = FrontmatterPreprocessor::default();

    // mdbook make two preprocessing requests:
    // 1) check that the renderer is supported
    // 2) pass JSON data [content, book] into stdin
    if args.len() > 2 && args[1] == "supports" {
        // Check if the preprocessor supports the specified renderer
        handle_supports(&preprocessor, args[2].as_str());
    } else {
        // Normal operation - process book contents
        if let Err(e) = preprocessor.handle_preprocessing() {
            eprintln!("Error processing frontmatter: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn handle_supports(pre: &dyn Preprocessor, renderer: &str) -> ! {
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

/// The actual implementation of the `FrontmatterPreprocessor` preprocessor. This would usually go
/// in the preprocessor's `lib.rs` file.
mod all_caps_lib {
    use super::*;
    use mdbook::BookItem;

    /// A preprocessor for doing things with frontmatter.
    #[derive(Default)]
    pub struct FrontmatterPreprocessor;

    impl FrontmatterPreprocessor {
        /// Preprocess book content.
        ///
        /// This method calls the impl `run` method for [Self] to edit content
        /// and return the processed [Book] to stdout.
        pub fn handle_preprocessing(&self) -> Result<(), Error> {
            let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;
            let book_version = Version::parse(&ctx.mdbook_version)?;
            let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

            if !version_req.matches(&book_version) {
                // log error
                eprintln!(
                    "Warning: The {} plugin was built against version {} of mdbook, \
                     but we're being called from version {}",
                    self.name(),
                    mdbook::MDBOOK_VERSION,
                    ctx.mdbook_version
                );
            }

            // process and return book to stdout
            let processed_book = self.run(&ctx, book)?;
            serde_json::to_writer(io::stdout(), &processed_book)?;
            Ok(())
        }

        /// Helper method to reformat date in frontmatter.
        ///
        /// This method ensures proper formatting before returning a new string.
        fn reformat_date(&self, date_str: &str) -> Result<String, &'static str> {
            let parts: Vec<&str> = date_str.split('-').collect();
            if parts.len() == 3 {
                // ensure expected format YYYY-MM-DD
                if parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2 {
                    // Assuming parts[0] is year, parts[1] is month, and parts[2] is day
                    return Ok(format!("{}-{}-{}", parts[1], parts[2], parts[0]));
                }
            }
            Err("Invalid date format")
        }
    }

    impl Preprocessor for FrontmatterPreprocessor {
        fn name(&self) -> &str {
            "frontmatter-preprocessor"
        }

        fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
            // loop through each book item to find chapters
            book.for_each_mut(|item| {
                if let BookItem::Chapter(chapter) = item {
                    println!("before: {:?}", chapter.frontmatter);
                    for (key, val) in chapter.frontmatter.iter_mut() {
                        // ensure all uppercase
                        *val = val.to_uppercase();

                        // format date as another example
                        if key == "date" {
                            *val = self.reformat_date(val).expect(&format!(
                                "date format incorrect. expected YYYY-MM-DD, received {}",
                                val
                            ));
                        }
                    }
                    println!("after: {:?}", chapter.frontmatter);
                }
            });

            Ok(book)
        }

        fn supports_renderer(&self, renderer: &str) -> bool {
            renderer != "not-supported"
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use mdbook::book::Chapter;

        #[test]
        fn frontmatter_preprocessor_run() {
            let input_json = r##"[
                {
                    "root": "/path/to/book",
                    "config": {
                        "book": {
                            "authors": ["AUTHOR"],
                            "language": "en",
                            "multilingual": false,
                            "src": "src",
                            "title": "TITLE"
                        },
                        "preprocessor": {
                            "nop": {}
                        }
                    },
                    "renderer": "html",
                    "mdbook_version": "0.4.21"
                },
                {
                    "sections": [
                        {
                            "Chapter": {
                                "name": "Chapter 1",
                                "content": "# Chapter 1\n",
                                "number": [1],
                                "sub_items": [],
                                "path": "chapter_1.md",
                                "source_path": "chapter_1.md",
                                "parent_names": [],
                                "frontmatter": {
                                    "author": "grant (@grantkee)",
                                    "date": "2024-08-02"
                                }
                            }
                        }
                    ],
                    "__non_exhaustive": null
                }
            ]"##;
            let input_json = input_json.as_bytes();

            let (ctx, book) = mdbook::preprocess::CmdPreprocessor::parse_input(input_json).unwrap();
            let result = FrontmatterPreprocessor::default().run(&ctx, book);
            let processed_book = result.expect("book processed");

            // only one section - chapter with frontmatter
            let BookItem::Chapter(ref chapter_1) = processed_book.sections[0] else {
                panic!("preprocessor changed BookItem variant")
            };
            let processed_frontmatter = &chapter_1.frontmatter;
            let expected_date = "08-02-2024";
            let expected_author = "GRANT (@GRANTKEE)";
            assert_eq!(processed_frontmatter["author"], expected_author);
            assert_eq!(processed_frontmatter["date"], expected_date);
        }
    }
}
