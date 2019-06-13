// Copyright (c) 2019 Weird Constructor <weirdconstructor@gmail.com>
// This is a part of gtp-rs. See README.md and COPYING for details.

/*!
A GTP (Go Text Protocol) controller implementation for Rust
===========================================================

This crate implements currently just a parser and serializer for
the Go Text Protocol as needed for writing a GTP controller to
control a GTP engine.

See also:

* [GTP (Go Text Protocol)](https://www.lysator.liu.se/~gunnar/gtp/)

# Usage

Sending commands:

```
use gtp;
let mut c = gtp::Command::new("list_commands");
assert_eq!(c.to_string(), "list_commands\n");
```

Sending commands with entities:

```
use gtp::Command;
assert_eq!(Command::new("clear_board").to_string(), "clear_board\n");
assert_eq!(Command::new_with_args("boardsize", |eb| eb.i(19)).to_string(),
           "boardsize 19\n");
```

Receiving Responses:

```
let mut rp = gtp::ResponseParser::new();
rp.feed("= o");
rp.feed("k\n\n");
rp.feed("= A\nB\nC\n\n= white b3 b T19\n\n");

assert_eq!(rp.get_response().unwrap().text(), "ok");
assert_eq!(rp.get_response().unwrap().text(), "A\nB\nC");

// And processing entities in the response:
let ents = rp.get_response().unwrap()
             .entities(|ep| ep.color().vertex().mv()).unwrap();
assert_eq!(ents[0].to_string(), "w");
assert_eq!(ents[1].to_string(), "B3");
assert_eq!(ents[2].to_string(), "b T19");

// And processing entities in the response more complicatedly:
rp.feed("= white b3\n\n");

let mut ep = gtp::EntityParser::new(&rp.get_response().unwrap().text());
let res = ep.mv().result().unwrap();
assert_eq!(res[0].to_string(), "w B3");

match res[0] {
    gtp::Entity::Move((color, (h, v))) => {
        assert_eq!(color, gtp::Color::W);
        assert_eq!(h, 2);
        assert_eq!(v, 3);
    },
    _ => {},
}
```

# Future

Currently I work on a GTP controller via tokio_process, as the dependency on tokio is quite heavy I
would not like to burden this little crate with that. But what I could see is a GTP controller
based on std::process which uses threads for communicating with the GTP engine in the background.

# License

This project is licensed under the GNU General Public License Version 3 or
later.

## Why GPL?

Picking a license for my code bothered me for a long time. I read many
discussions about this topic. Read the license explanations. And discussed
this matter with other developers.

First about _why I write code for free_ at all:

- It's my passion to write computer programs. In my free time I can
write the code I want, when I want and the way I want. I can freely
allocate my time and freely choose the projects I want to work on.
- To help a friend or member of my family.
- To solve a problem I have.

Those are the reasons why I write code for free. Now the reasons
_why I publish the code_, when I could as well keep it to myself:

- So that it may bring value to users and the free software community.
- Show my work as an artist.
- To get into contact with other developers.
- And it's a nice change to put some more polish on my private projects.

Most of those reasons don't yet justify GPL. The main point of the GPL, as far
as I understand: The GPL makes sure the software stays free software until
eternity. That the user of the software always stays in control. That the users
have _at least the means_ to adapt the software to new platforms or use cases.
Even if the original authors don't maintain the software anymore.
It ultimately prevents _"vendor lock in"_. I really dislike vendor lock in,
especially as developer. Especially as developer I want and need to stay
in control of the computers I use.

Another point is, that my work has a value. If I give away my work without
_any_ strings attached, I effectively work for free. Work for free for
companies. I would compromise the price I can demand for my skill, workforce
and time.

This makes two reasons for me to choose the GPL:

1. I do not want to support vendor lock in scenarios. At least not for free.
   I want to prevent those when I have a choice.
   And before you ask, yes I work for a company that sells closed source
   software. I am not happy about the closed source fact.
   But it pays my bills and gives me the freedom to write free software
   in my free time.
2. I don't want to low ball my own wage and prices by giving away free software
   with no strings attached (for companies).

## If you need a permissive or private license (MIT)

Please contact me if you need a different license and really want to use
my code. As long as I am the only author, I can change the license.
We might find an agreement.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in WLambda by you, shall be licensed as GPLv3 or later,
without any additional terms or conditions.

# Authors

* Weird Constructor <weirdconstructor@gmail.com>
  (You may find me as `WeirdConstructor` on the Rust Discord.)

*/

mod controller;

/// The color of a move
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    W,
    B,
}

/// Helper class for constructing an Entity data structure.
///
/// Use it like this:
/// ```
/// use gtp;
/// let mut eb = gtp::EntityBuilder::new();
/// eb.v((19, 19));
/// assert_eq!(eb.build().to_string(), "T19");
/// ```
///
/// Alternatively you can use the [`entity`](fn.entity.html) function:
///
/// ```
/// use gtp;
/// let ent = gtp::entity(|eb| eb.v((19, 19)));
/// assert_eq!(ent.to_string(), "T19");
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntityBuilder {
    list:    Vec<Entity>,
    current: Option<Entity>,
}

/// Entity building helper function.
///
/// ```
/// use gtp;
/// assert_eq!(gtp::entity(|eb| eb.v_pass()).to_string(), "pass");
/// ```
pub fn entity<T>(f: T) -> Entity
    where T: Fn(&mut EntityBuilder) -> &mut EntityBuilder {
    let mut b = EntityBuilder::new();
    f(&mut b);
    b.build()
}

impl EntityBuilder {
    /// Constructs a new entity builder.
    ///
    /// Please note there are helper functions like [`entity`](fn.entity.html)
    /// Or [`args` of Command](struct.Command.html#method.args).
    pub fn new() -> EntityBuilder {
        EntityBuilder::default()
    }

    pub fn i(&mut self, i: u32) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Int(i));
        self
    }

    pub fn f(&mut self, f: f32) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Float(f));
        self
    }

    pub fn s(&mut self, s: &str) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::String(s.to_string()));
        self
    }

    pub fn v_pass(&mut self) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Vertex((0, 0)));
        self
    }

    pub fn v(&mut self, v: (i32, i32)) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Vertex(v));
        self
    }

    pub fn w(&mut self) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Color(Color::W));
        self
    }

    pub fn b(&mut self) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Color(Color::B));
        self
    }

    pub fn bool(&mut self, b: bool) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Boolean(b));
        self
    }

    pub fn color(&mut self, b: bool) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Color(if b { Color::W } else { Color::B }));
        self
    }

    pub fn mv_w(&mut self, v: (i32, i32)) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Move((Color::W, v)));
        self
    }

    pub fn mv_b(&mut self, v: (i32, i32)) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Move((Color::B, v)));
        self
    }

    pub fn mv(&mut self, color: bool, v: (i32, i32)) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::Move((if color { Color::W } else { Color::B }, v)));
        self
    }

    pub fn list(&mut self) -> &mut Self {
        if self.current.is_some() { self.list.push(self.current.take().unwrap()); }

        self.current = Some(Entity::List(self.list.clone()));
        self.list = Vec::new();
        self
    }

    pub fn build(&self) -> Entity {
        self.current.clone().expect("Did not setup any entitiy in EntityBuilder!")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    Int(u32),
    Float(f32),
    String(String),
    Vertex((i32, i32)),
    Color(Color),
    Move((Color, (i32, i32))),
    Boolean(bool),
    List(Vec<Entity>),
}

fn gen_move_char(i: u32) -> char {
    let c = if i <= 8 {
        ('A' as u32) + (i - 1)
    } else {
        ('A' as u32) + i
    };
    if let Some(c) = std::char::from_u32(c) {
        c
    } else {
        'Z'
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Entity::Int(i)      => write!(f, "{}", i),
            Entity::Float(n)    => write!(f, "{}", n),
            Entity::String(s)   => write!(f, "{}", s),
            Entity::Vertex((h, v)) => {
                let mut s = String::from("");
                if *h <= 0 || *v <= 0 {
                    s += &"pass".to_string();
                } else {
                    s += &format!("{}", gen_move_char(*h as u32));
                    s += &format!("{}", v);
                }
                write!(f, "{}", s)
            },
            Entity::Color(Color::W) => write!(f, "w"),
            Entity::Color(Color::B) => write!(f, "b"),
            Entity::Move((Color::W, (h, v))) => {
                let mut s = String::from("");
                if *h <= 0 || *v <= 0 {
                    s += &"w pass".to_string();
                } else {
                    s += &format!("w {}", gen_move_char(*h as u32));
                    s += &format!("{}", v);
                }
                write!(f, "{}", s)
            },
            Entity::Move((Color::B, (h, v))) => {
                let mut s = String::from("");
                if *h <= 0 || *v <= 0 {
                    s += &"b pass".to_string();
                } else {
                    s += &format!("b {}", gen_move_char(*h as u32));
                    s += &format!("{}", v);
                }
                write!(f, "{}", s)
            },
            Entity::Boolean(true) => write!(f, "true"),
            Entity::Boolean(false) => write!(f, "false"),
            Entity::List(vec) => {

                let mut s = String::from("");
                if vec.is_empty() { return write!(f, ""); }

                // Try to handle at least 2 dimensional lists
                // on output correctly:
                let sep = if let Entity::List(_) = vec[0] {
                    "\n"
                } else {
                    " "
                };

                for (i, e) in vec.iter().enumerate() {
                    if i > 0 { s += sep; }
                    s += &e.to_string();
                }
                write!(f, "{}", s)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityParser {
    buffer:         String,
    entities:       Vec<Entity>,
    parse_error:    bool,
}

impl std::iter::Iterator for EntityParser {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.buffer = self.buffer.chars().skip_while(|c| *c == ' ' || *c == '\n').collect();

        let mut s = String::from("");
        let mut skip_count = 0;
        for c in self.buffer.chars() {
            skip_count += 1;
            if c == ' ' || c == '\n' { break; }
            s.push(c);
        }

        self.buffer = self.buffer.chars().skip(skip_count).collect();

        if s.is_empty() { None } else { Some(s) }
    }

}

impl EntityParser {
    pub fn new(s: &str) -> Self {
        EntityParser {
            buffer:     String::from(s),
            entities:   Vec::new(),
            parse_error: false,
        }
    }

    pub fn result(&self) -> Option<Vec<Entity>> {
        if self.parse_error { return None; }
        Some(self.entities.clone())
    }

    pub fn is_eof(&self) -> bool { self.buffer.is_empty() }
    pub fn had_parse_error(&self) -> bool { self.parse_error }

    pub fn s(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        if s.is_empty() { self.parse_error = true; return self; }
        self.entities.push(Entity::String(s));
        self
    }

    pub fn i(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        if let Ok(i) = s.parse::<u32>() {
            self.entities.push(Entity::Int(i));
        } else {
            self.parse_error = true;
        }
        self
    }

    pub fn f(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        if let Ok(f) = s.parse::<f32>() {
            self.entities.push(Entity::Float(f));
        } else {
            self.parse_error = true;
        }
        self
    }

    pub fn color(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        let s = s.to_lowercase();
        if s == "w" || s == "white" { self.entities.push(Entity::Color(Color::W)); return self; }
        if s == "b" || s == "black" { self.entities.push(Entity::Color(Color::B)); return self; }
        self.parse_error = true;
        self
    }

    pub fn vertex(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        let s = s.to_uppercase();
        if s == "PASS" { self.entities.push(Entity::Vertex((0, 0))); return self; }
        if s.len() < 2 || s.len() > 3 {
            self.parse_error = true;
            return self;
        }

        let h = s.chars().nth(0).unwrap();
        if !h.is_ascii_alphabetic() {
            self.parse_error = true;
            return self;
        }
        let h = h as u32;
        let mut h = (h - ('A' as u32)) + 1;
        if h > 8 { h -= 1; }

        let v : String = s.chars().skip(1).collect();
        if let Ok(v) = i32::from_str_radix(&v, 10) {
            self.entities.push(Entity::Vertex((h as i32, v)));
        } else {
            self.parse_error = true;
        }

        self
    }

    pub fn mv(&mut self) -> &mut Self {
        self.color();
        if self.parse_error { return self; }
        self.vertex();
        if self.parse_error { self.entities.pop(); }

        let m = self.entities.pop().unwrap();
        let c = self.entities.pop().unwrap();

        if let Entity::Vertex((h, v)) = m {
            if let Entity::Color(c) = c {
                self.entities.push(Entity::Move((c, (h, v))));
                return self;
            }
        }

        self.parse_error = true;
        self
    }

    pub fn bool(&mut self) -> &mut Self {
        let s = self.next().unwrap_or_else(|| String::from(""));
        let s = s.to_uppercase();
        if s == "TRUE" { self.entities.push(Entity::Boolean(true)); return self; }
        if s == "FALSE" { self.entities.push(Entity::Boolean(false)); return self; }
        self.parse_error = true;
        self
    }
}

/// Representation of a GTP controller to engine command.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    id:     Option<u32>,
    name:   String,
    args:   Option<Entity>,
}

impl Command {
    /// Constructs a new GTP controller command to be sent to the
    /// GTP engine.
    ///
    /// ```
    /// use gtp;
    ///
    /// let mut c = gtp::Command::new("list_commands");
    /// c.args(|eb| eb.i(10).f(10.20).s("OK").list());
    /// // send with:
    /// let gtp_bytes = c.to_bytes();
    /// // or if you need the string:
    /// let gtp_str = c.to_string();
    /// ```
    pub fn new(name: &str) -> Command {
        Command {
            name: String::from(name),
            id:   None,
            args: None,
        }
    }

    /// Builds a new GTP engine command ready with the arguments:
    ///
    /// ```
    /// use gtp;
    /// assert_eq!(
    ///     gtp::Command::new_with_args("boardsize", |eb| eb.i(9)).to_string(),
    ///     "boardsize 9\n");
    /// ```
    pub fn new_with_args<T>(name: &str, args: T) -> Command
        where T: Fn(&mut EntityBuilder) -> &mut EntityBuilder {
        let mut cmd = Self::new(name);
        cmd.args(args);
        cmd
    }

    /// Shorthand for `Command::new_with_args`.
    pub fn cmd<T>(name: &str, args: T) -> Command
        where T: Fn(&mut EntityBuilder) -> &mut EntityBuilder {
        new_with_args(name, args)
    }

    /// Sets the ID of the command.
    ///
    /// ```
    /// use gtp;
    ///
    /// let mut c = gtp::Command::new("list_commands");
    /// c.set_id(12);
    /// assert_eq!(c.to_string(), "12 list_commands\n");
    /// ```
    pub fn set_id(&mut self, id: u32) {
        self.id = Some(id);
    }

    /// Helper function to construct Entity arguments for this Command.
    ///
    /// ```
    /// use gtp;
    ///
    /// let mut c = gtp::Command::new("list_commands");
    /// c.args(|eb| eb.i(10).f(10.20).s("OK").list());
    /// assert_eq!(c.to_string(), "list_commands 10 10.2 OK\n");
    /// ```
    pub fn args<T>(&mut self, f: T)
        where T: Fn(&mut EntityBuilder) -> &mut EntityBuilder {
        let mut b = EntityBuilder::new();
        f(&mut b);
        self.set_args(&b.build());
    }


    /// Function to set Entity arguments:
    ///
    /// ```
    /// use gtp;
    ///
    /// let mut c = gtp::Command::new("list_commands");
    /// c.set_args(&gtp::entity(|eb| eb.v((19, 19))));
    /// ```
    pub fn set_args(&mut self, args: &Entity) {
        self.args = Some(args.clone());
    }

    /// Generates a String representation of the GTP command.
    pub fn to_string(&self) -> String {
        let mut out = String::from("");
        if self.id.is_some() {
            out += &format!("{}", self.id.unwrap());
            out += " ";
        }
        out += &self.name;

        if self.args.is_some() {
            out += " ";
            out += &self.args.as_ref().unwrap().to_string();
        }
        out += "\n";
        out
    }

    /// Generates a byte vector representation of the GTP command,
    /// ready to be sent to another process.
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        Vec::from(self.to_string().as_bytes())
    }
}

/// Represents a GTP response from the GTP engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Error((Option<u32>, String)),
    Result((Option<u32>, String)),
}

#[derive(Debug)]
pub enum ResponseParseError {
    NoInput,
    BadEntityInput,
    BadResponse
}

impl Response {
    /// Returns the complete response text, which you may feed into EntityParser.
    ///
    /// See also the `Response::entities` method.
    pub fn text(&self) -> String {
        match self {
            Response::Error((_, t))  => t.clone(),
            Response::Result((_, t)) => t.clone(),
        }
    }

    /// Returns the ID of the response. Returns 0 if no
    /// ID was submitted.
    pub fn id_0(&self) -> u32 {
        match self {
            Response::Error((None, _))          => 0,
            Response::Result((None, _))         => 0,
            Response::Error((Some(id), _))      => *id,
            Response::Result((Some(id), _))     => *id,
        }
    }

    /// Parses entities from a Response
    ///
    /// ```
    /// let mut rp = gtp::ResponseParser::new();
    /// rp.feed("= 10 w H6\n\n");
    /// let entity_vec =
    ///     rp.get_response().unwrap()
    ///       .entities(|ep| ep.i().mv())
    ///       .unwrap();
    /// assert_eq!(format!("{:?}", entity_vec),
    ///            "[Int(10), Move((W, (8, 6)))]");
    ///
    /// if let gtp::Entity::Int(i) = entity_vec[0] {
    ///     assert_eq!(i, 10);
    /// }
    /// ```
    ///
    /// Here is an example for how to read a variable length list:
    ///
    /// ```
    /// let mut rp = gtp::ResponseParser::new();
    /// rp.feed("= A\nB\nC\nD\nE\n\n");
    /// let entity_vec =
    ///     rp.get_response().unwrap()
    ///       .entities(|ep| { while !ep.is_eof() { ep.s(); }; ep })
    ///       .unwrap();
    /// assert_eq!(format!("{:?}", entity_vec),
    ///            "[String(\"A\"), String(\"B\"), String(\"C\"), String(\"D\"), String(\"E\")]");
    /// ```
    pub fn entities<T>(&self, parse_fn: T) -> Result<Vec<Entity>, ResponseParseError>
        where T: Fn(&mut EntityParser) -> &mut EntityParser  {

        let response = match self {
            Response::Result((_, res)) => res.to_string(),
            Response::Error((_, res))  => res.to_string(),
        };

        let mut ep = EntityParser::new(&response);
        parse_fn(&mut ep);
        if ep.had_parse_error() {
            return Err(ResponseParseError::BadEntityInput);
        }
        Ok(ep.result().unwrap())
    }
}

/// A parser for a GTP response.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResponseParser {
    buffer:     String,
}

/// Error for the ResponseParser.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseError {
    IncompleteResponse,
    BadResponse(String),
}

fn refine_input(s: String) -> String {
    let mut ret : String =
        s.chars()
         .filter(|c| *c != '\r')
         .map(|c| if c == '\x09' { ' ' } else { c })
         .skip_while(|c| *c == '\n' || *c == ' ' || *c == '\x09')
         .collect();

    loop {
        let comment_pos = ret.find('#');
        if comment_pos.is_some() {
            let end_comment_pos = (&ret[comment_pos.unwrap()..]).find('\n');
            if end_comment_pos.is_some() {
                ret = String::from(&ret[..comment_pos.unwrap()])
                      + &ret[comment_pos.unwrap() + end_comment_pos.unwrap() + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }

    ret
}

impl ResponseParser {
    /// Constructs a new GTP engine response parser.
    ///
    /// ```
    /// let mut rp = gtp::ResponseParser::new();
    /// rp.feed("= ok\n\n");
    /// let s = rp.get_response().unwrap();
    /// assert_eq!(format!("{:?}", s), "Result((None, \"ok\"))");
    /// ```
    pub fn new() -> ResponseParser {
        ResponseParser::default()
    }

    /// Feed the response text to the parser.
    pub fn feed(&mut self, s: &str) {
        self.buffer += s;
    }

    /// Tries to read the response from the until now feeded input.
    ///
    /// Returns `Ok(None)` if no response is available yet.
    /// Returns an error if the response is malformed.
    /// Returns the Ok([`Response`](enum.Response.html)) if one could be read.
    #[allow(unused_assignments, clippy::collapsible_if)]
    pub fn get_response(&mut self) -> Result<Response, ResponseError> {
        self.buffer = refine_input(self.buffer.to_string());
        if self.buffer.is_empty() { return Err(ResponseError::IncompleteResponse); }

        let is_error = self.buffer.chars().nth(0).unwrap() != '=';

        let mut id_str   = String::from("");
        let mut response = String::from("");

        let mut read_id =
            !(   self.buffer.len() > 1
              && self.buffer.chars().nth(1).unwrap() == ' ');

        let mut found_start      = false;
        let mut found_end        = false;
        let mut last_was_newline = false;
        let mut skip_count       = 1;

        for c in self.buffer.chars().skip(1) {
            skip_count += 1;

            if read_id {
                match c {
                    c if c.is_ascii_digit() => {
                        id_str.push(c);
                    },
                    ' ' => {
                        found_start = true;
                        read_id     = false;
                    },
                    _ => { return Err(ResponseError::BadResponse(self.buffer.to_string())); }
                }
            } else if !found_start {
                if c == ' ' {
                    found_start = true;
                } else {
                    return Err(ResponseError::BadResponse(self.buffer.to_string()));
                }
            } else {
                if c == '\n' {
                    if last_was_newline {
                        found_end = true;
                        break;
                    } else {
                        last_was_newline = true;
                    }
                } else {
                    if last_was_newline {
                        response.push('\n');
                    }
                    last_was_newline = false;
                    response.push(c);
                }
            }
        }

        if found_end {
            self.buffer = self.buffer.chars().skip(skip_count).collect();
        } else {
            return Err(ResponseError::IncompleteResponse);
        }

        let id = if !id_str.is_empty() {
            if let Ok(cn) = u32::from_str_radix(&id_str, 10) {
                Some(cn)
            } else {
                None
            }
        } else {
            None
        };

        if is_error {
            Ok(Response::Error((id, response)))
        } else {
            Ok(Response::Result((id, response)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_printing() {
        assert_eq!(Entity::Int(10).to_string(),                      "10");
        assert_eq!(Entity::Float(10.12).to_string(),                 "10.12");
        assert_eq!(Entity::String(String::from("Test")).to_string(), "Test");
        assert_eq!(Entity::Vertex((-1, -1)).to_string(),             "pass");
        assert_eq!(Entity::Vertex((1, 1)).to_string(),               "A1");
        assert_eq!(Entity::Vertex((19, 19)).to_string(),             "T19");
        assert_eq!(Entity::Vertex((8, 19)).to_string(),              "H19");
        assert_eq!(Entity::Vertex((9, 19)).to_string(),              "J19");
        assert_eq!(Entity::Color(Color::W).to_string(),              "w");
        assert_eq!(Entity::Color(Color::B).to_string(),              "b");
        assert_eq!(Entity::Move((Color::B, (0, 0))).to_string(),     "b pass");
        assert_eq!(Entity::Move((Color::W, (0, 0))).to_string(),     "w pass");
        assert_eq!(Entity::Move((Color::B, (8, 1))).to_string(),     "b H1");
        assert_eq!(Entity::Move((Color::W, (9, 1))).to_string(),     "w J1");
        assert_eq!(Entity::Move((Color::B, (19, 1))).to_string(),    "b T1");
        assert_eq!(Entity::Move((Color::W, (19, 19))).to_string(),   "w T19");
        assert_eq!(Entity::Boolean(true).to_string(),                "true");
        assert_eq!(Entity::Boolean(false).to_string(),               "false");
        assert_eq!(Entity::List(vec![Entity::Int(1), Entity::Int(2)]).to_string(),
                   "1 2");
        assert_eq!(Entity::List(vec![
                        Entity::List(vec![Entity::Int(1), Entity::Int(2)]),
                        Entity::List(vec![Entity::Int(3), Entity::Int(4)])]).to_string(),
                   "1 2\n3 4");
    }

    #[test]
    fn check_entity_builder() {
        assert_eq!(entity(|eb| eb.i(10)).to_string(),           "10");
        assert_eq!(entity(|eb| eb.f(10.12)).to_string(),        "10.12");
        assert_eq!(entity(|eb| eb.s("ok")).to_string(),         "ok");
        assert_eq!(entity(|eb| eb.v_pass()).to_string(),        "pass");
        assert_eq!(entity(|eb| eb.v((19, 19))).to_string(),     "T19");
        assert_eq!(entity(|eb| eb.bool(false)).to_string(),     "false");
        assert_eq!(entity(|eb| eb.w()).to_string(),             "w");
        assert_eq!(entity(|eb| eb.b()).to_string(),             "b");
        assert_eq!(entity(|eb| eb.color(true)).to_string(),     "w");
        assert_eq!(entity(|eb| eb.color(false)).to_string(),    "b");
        assert_eq!(entity(|eb| eb.mv_w((8, 8))).to_string(),    "w H8");
        assert_eq!(entity(|eb| eb.mv_b((8, 8))).to_string(),    "b H8");
        assert_eq!(entity(|eb| eb.mv(true, (8, 8))).to_string(),"w H8");
        assert_eq!(entity(|eb| eb.mv_w((8, 8)).mv_b((9, 9)).list()).to_string(),
                   "w H8 b J9");
    }

    #[test]
    fn check_entity_parser() {
        let mut ep = EntityParser::new("10 10.2 ok WHite t19 false");
        ep.i().f().s().mv().bool();
        let res = ep.result().unwrap();
        assert_eq!(res[0].to_string(), "10");
        assert_eq!(res[1].to_string(), "10.2");
        assert_eq!(res[2].to_string(), "ok");
        assert_eq!(res[3].to_string(), "w T19");
        assert_eq!(res[4].to_string(), "false");
    }

    #[test]
    fn check_eof() {
        let mut ep = EntityParser::new("t19 b10 a1 d2");
        while !ep.is_eof() {
            ep.vertex();
        }
        let res = ep.result().unwrap();
        assert_eq!(res[3].to_string(), "D2");
    }

    #[test]
    fn check_build_command() {
        let mut c = Command::new("list_commands");
        c.args(|eb| eb.i(10).f(10.20).s("OK").list());
        assert_eq!(c.to_string(), "list_commands 10 10.2 OK\n");

        assert_eq!(
            Command::new_with_args("boardsize", |eb| eb.i(9)).to_string(),
            "boardsize 9\n");
    }

    #[test]
    fn check_setid_command() {
        let mut c = Command::new("list_commands");
        c.set_id(12);
        assert_eq!(c.to_string(), "12 list_commands\n");
    }

    fn must_parse(s: &str) -> Response {
        let mut rp = ResponseParser::new();
        rp.feed(s);
        let s = rp.get_response().unwrap();
        s
    }

    #[test]
    fn check_parser() {
        {
            let mut rp = ResponseParser::new();
            rp.feed("= ok\n\n");
            let s = rp.get_response().unwrap();
            assert_eq!(format!("{:?}", s), "Result((None, \"ok\"))");
        }

        {
            let mut rp = ResponseParser::new();
            rp.feed("= ok\n\n");
            rp.feed("= \n\n");

            assert_eq!(rp.get_response().unwrap().text(), "ok");
        }

        {
            let mut rp = ResponseParser::new();
            rp.feed("= ok\n");

            assert!(rp.get_response().is_err());
        }

        let res = must_parse("= ok\nfoobar\n\n");
        assert_eq!(res.text(), "ok\nfoobar");

        assert_eq!(format!("{:?}", must_parse("=10 ok\n\n")),
                   "Result((Some(10), \"ok\"))");

        assert_eq!(format!("{:?}", must_parse("#\n=10 ok\n\n")),
                   "Result((Some(10), \"ok\"))");

        assert_eq!(format!("{:?}", must_parse("= ok\n\n")),
                   "Result((None, \"ok\"))");

        assert_eq!(format!("{:?}", must_parse("= \n\n")),
                   "Result((None, \"\"))");

        assert_eq!(format!("{:?}", must_parse("= \na\nb\nc\n\n")),
                   "Result((None, \"\\na\\nb\\nc\"))");

        assert_eq!(format!("{:?}", must_parse("= foo # all ok\n\n\n")),
                   "Result((None, \"foo \"))");

        assert_eq!(format!("{:?}", must_parse("= \na\nb fooo #fewiofw jfw\nc\n\n")),
                   "Result((None, \"\\na\\nb fooo c\"))");
    }

}
