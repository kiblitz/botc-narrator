open! Core

module Kind = struct
  type t =
    | Townsfolk
    | Outsider
    | Minion
    | Demon
  [@@deriving sexp, equal, compare]
end

module Alignment = struct
  type t =
    | Good
    | Evil
  [@@deriving sexp, equal, compare]
end
