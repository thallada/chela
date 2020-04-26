# chela: HTML & CSS Sanitizer and Transformer

![chela](img/chela.svg)

chela (/ˈkiːlə/ — KEE-LUH) is a program that prunes untrusted HTML and CSS using 
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

## Why chela?

At the time of writing, [ammonia](https://github.com/rust-ammonia/ammonia) is 
the most popular and battle-tested HTML sanitization library written in Rust. In 
most cases, it should be used over this library. However, chela provides 
sanitization of CSS in addition to HTML, which ammonia does not support. Also, 
chela allows users to write custom functions to perform more complex 
transformations that simple whitelist rules cannot support. In this way, chela 
is more than a sanitization library, but a tool to rapidly perform 
transformations on HTML and CSS inputs.