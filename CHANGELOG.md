0.1.2 (2021-08-11)
==================

* Feature: Added gtp::controller and fixed the test cases.
* Change: Improved documentation.

0.1.1 (2019-06-14)
==================

* Incompatible change: gtp::ResponseParser::get\_response returns Result<> now.
* Incompatible change: gtp::Error renamed to gtp::ResponseError.
* Feature: Added gtp::Response::entities() for parsing response entities.
* Feature: Added detached\_command module for starting processes and
  communicate with them non-blocking.
* Change: Implement ToString and Display traits for Entity.
* Bugfix: Parsing was broken, involving comments.

0.1.0 (2019-06-12)
==================

* Initial version.
