# chela: HTML & CSS Sanitizer and Transformer

<p align="center">
  <img width="200" height="200" src="img/chela.svg">
</p>

chela (/ˈkiːlə/ - KEE-LUH) is a program that prunes untrusted HTML and CSS using 
a whitelist of rules. It is also a library for general-purpose HTML and CSS 
transforming that allows users to define custom functions that modify the parsed 
HTML tree node-by-node as it is traversed.

**This is still an experimental project. Use in production environments at your 
own risk.**

chela is heavily inspired by the Ruby project 
[sanitize](https://github.com/rgrove/sanitize). The goal of chela is to match 
the ease and usability of sanitize but with the performance and reliability of 
Rust under the hood. The browser-grade 
[html5ever](https://github.com/servo/html5ever) HTML parser and 
[rust-cssparser](https://github.com/servo/rust-cssparser) are used to parse HTML 
and CSS respectively.

This project expands on [an example in the html5ever 
repo](https://github.com/servo/html5ever/blob/7efca84c788bf9c9b4f314482b9630130812f994/html5ever/examples/arena.rs) 
which parses the HTML tree into a cyclic node structure allocated in an arena 
using the [typed-arena](https://github.com/SimonSapin/rust-typed-arena) crate.
Allocating into the arena is not only very fast, but gets around tricky 
borrow-checking issues in Rust to enable bi-directional tree structures that 
provide the most flexibility in traversing the tree (being able to look up the 
parents *and* the children of any given node).

## Why chela?

At the time of writing, [ammonia](https://github.com/rust-ammonia/ammonia) is 
the most popular and battle-tested HTML sanitization library written in Rust. In 
most cases, it should be used over this library. However, chela provides 
sanitization of CSS in addition to HTML, which ammonia does not support. Also, 
chela allows users to write custom functions to perform more complex 
transformations that simple whitelist rules cannot support. In this way, chela 
is more than a sanitization library, it's tool to rapidly perform 
transformations on HTML and CSS inputs.

## TODO

There is still a lot left to be done with this project. Here's a high-level 
overview of what I think is left.

* Proper binary and library distinction: I need to add a proper command-line 
  interface as well as solidify the public API I'd like to expose to users of 
  the library.
* Documentation: I won't consider this project usable until at least that is 
  done. I plan on using the standard Rust cargo docs for this.
* First release on crates.io
* Examples: I would like a bunch of short examples in this README highlighting 
  the features of chela in addition to an `examples` folder with more complex 
  examples.
    - One example in particular that I'd like to cover is using chela from a 
      Rails project using [Helix](https://github.com/tildeio/helix). A major 
      motivation for this project was to provide a speedy drop-in replacement 
      for santize in Ruby projects.
* More tests.
    - Unit tests: I have some of these already, but more modules need it. I also 
      would like to find a better method of organizing them.
    - Test cases from ammonia and sanitize: I'd like a guarantee that chela is 
      as safe to use as ammonia or sanitize. I can only get to that point once 
      I've imported all of the input/output tests from those two projects.
* CI: A way to automate tests. This would be a necessary step towards providing 
  a streamlined process for community pull requests.
* Benchmarks: I'd like to know how fast this library compares to similar 
  projects like ammonia and sanitize. If it's faster, that would be a great 
  thing to know, if it's slower I'd like to do some profiling to find areas to 
  improve.
