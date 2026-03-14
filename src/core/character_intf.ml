open! Core

type kind      = Townsfolk | Outsider | Minion | Demon [@@deriving sexp, equal, compare]
type alignment = Good      | Evil                      [@@deriving sexp, equal, compare]

(** Static, monad-free properties of a character. *)
module type S = sig
  (** Snake-case unique identifier, e.g. ["imp"], ["fortune_teller"]. *)
  val id        : string
  val name      : string
  val kind      : kind
  val alignment : alignment
end

type t = (module S)

let id        (module C : S) = C.id
let name      (module C : S) = C.name
let kind      (module C : S) = C.kind
let alignment (module C : S) = C.alignment

let equal   a b = String.equal   (id a) (id b)
let compare a b = String.compare (id a) (id b)

let is_townsfolk c = equal_kind      (kind c) Townsfolk
let is_outsider  c = equal_kind      (kind c) Outsider
let is_minion    c = equal_kind      (kind c) Minion
let is_demon     c = equal_kind      (kind c) Demon
let is_evil      c = equal_alignment (alignment c) Evil
let is_good      c = equal_alignment (alignment c) Good
