open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "baron"
    let name = "Baron"
    let t = { kind = Kind.Minion; alignment = Alignment.Evil }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:_ ~night:_ = None
let day_action ~player_id:_ = None

let on_setup ~player_id:_ =
  Some (Botc_exec.log "Baron is in play: replace 2 Townsfolk with Outsiders")
;;

let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
