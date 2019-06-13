0.1.1 (2019-06-14)
==================

* Incompatible change: gtp::ResponseParser::get_response returns Result<> now.
* Incompatible change: gtp::Error renamed to gtp::ResponseError.
* Feature:             added gtp::Response::entities() for parsing response entities.
* Bugfix:              Parsing was broken, involving comments.

0.1.0 (2019-06-12)
==================

* Initial version.
