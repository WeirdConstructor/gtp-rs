A GTP (Go Text Protocol) controller implementation for Rust
===========================================================

This crate implements a parser, serializer for the Go Text Protocol and an
abstraction for an GTP engine controller.  You may just use the protocol
handling parts if you have an alternative GTP engine controller implementation.
But you are free to use the one provided by this crate.

See also:

* [GTP (Go Text Protocol)](https://www.lysator.liu.se/~gunnar/gtp/)

# Usage

A short overview of the API of this crate.

## Talking with a GTP engine

This is the basic usage on how to communicate with a GTP
engine like `GNU Go`, `Leela Zero` or `KataGo`:

```
use std::time::Duration;
use gtp::Command;
use gtp::controller::Engine;

let mut ctrl = Engine::new("/usr/bin/gnugo", &["--mode", "gtp"]);
assert!(ctrl.start().is_ok());

ctrl.send(Command::cmd("name", |e| e));
let resp = ctrl.wait_response(Duration::from_millis(500)).unwrap();
let ev = resp.entities(|ep| ep.s().s()).unwrap();
assert_eq!(ev[0].to_string(), "GNU");
assert_eq!(ev[1].to_string(), "Go");
assert_eq!(resp.text(), "GNU Go");
```

## Encoding GTP commands and Decoding GTP responses

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
for inclusion in gtp-rs by you, shall be licensed as GPLv3 or later,
without any additional terms or conditions.

# Authors

* Weird Constructor <weirdconstructor@gmail.com>
  (You may find me as `WeirdConstructor` on the Rust Discord.)
