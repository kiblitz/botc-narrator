open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "mayor"
    let name = "Mayor"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:_ ~night:_ = None
let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None

let on_night_kill ~player_id:pid =
  Some
    (let%bind.Botc_exec state = Botc_exec.get_state () in
     let candidates = alive_except state pid in
     let%bind.Botc_exec targets =
       Botc_exec.narrator_pick "mayor redirect target" candidates ~pick_count:1
     in
     let target = List.hd_exn targets in
     Botc_exec.modify_state (fun s -> Game_state.kill s target))
;;
