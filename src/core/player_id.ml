open! Core

module T = struct
  type t = string [@@deriving sexp, compare, equal]
end

include T
include Comparable.Make (T)
include (String : Stringable.S with type t := t)
