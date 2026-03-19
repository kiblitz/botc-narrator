open! Core

module Kind : sig
  type t =
    | Townsfolk
    | Outsider
    | Minion
    | Demon
  [@@deriving sexp, equal, compare]
end

module Alignment : sig
  type t =
    | Good
    | Evil
  [@@deriving sexp, equal, compare]
end
