open! Core

type t =
  | Townsfolk
  | Outsider
  | Minion
  | Demon
[@@deriving sexp, equal, compare]
