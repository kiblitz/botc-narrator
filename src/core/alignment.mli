open! Core

type t =
  | Good
  | Evil
[@@deriving sexp, equal, compare]
