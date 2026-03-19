open! Core

let all : (module Character_intf.S) list =
  [ (module Baron)
  ; (module Butler)
  ; (module Chef)
  ; (module Drunk)
  ; (module Empath)
  ; (module Fortune_teller)
  ; (module Imp)
  ; (module Investigator)
  ; (module Librarian)
  ; (module Mayor)
  ; (module Monk)
  ; (module Poisoner)
  ; (module Ravenkeeper)
  ; (module Recluse)
  ; (module Saint)
  ; (module Scarlet_woman)
  ; (module Slayer)
  ; (module Soldier)
  ; (module Spy)
  ; (module Undertaker)
  ; (module Virgin)
  ; (module Washerwoman)
  ]
;;

let night_one_order : (module Character_intf.S) list =
  [ (module Poisoner)
  ; (module Washerwoman)
  ; (module Librarian)
  ; (module Investigator)
  ; (module Chef)
  ; (module Empath)
  ; (module Fortune_teller)
  ; (module Butler)
  ]
;;

let night_subsequent_order : (module Character_intf.S) list =
  [ (module Poisoner)
  ; (module Monk)
  ; (module Imp)
  ; (module Ravenkeeper)
  ; (module Undertaker)
  ; (module Spy)
  ; (module Empath)
  ; (module Fortune_teller)
  ; (module Butler)
  ]
;;
