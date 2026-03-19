open! Core

type t =
  { id : string
  ; name : string
  ; kind : Kind.t
  ; alignment : Alignment.t
  }
[@@deriving fields ~getters, sexp_of]

let is_townsfolk t = Kind.equal t.kind Kind.Townsfolk
let is_outsider t = Kind.equal t.kind Kind.Outsider
let is_minion t = Kind.equal t.kind Kind.Minion
let is_demon t = Kind.equal t.kind Kind.Demon
let is_evil t = Alignment.equal t.alignment Alignment.Evil
let is_good t = Alignment.equal t.alignment Alignment.Good
