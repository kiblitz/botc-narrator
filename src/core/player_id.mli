open! Core

type t [@@deriving sexp, equal]

include Comparable.S with type t := t
include Stringable.S with type t := t
