open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "poisoner"
    let name = "Poisoner"
    let t = { kind = Kind.Minion; alignment = Alignment.Evil }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night:_ =
  Some
    (if_alive
       pid
       (let%bind.Botc_exec state = Botc_exec.get_state
        and () = Botc_exec.wake pid in
        let%bind.Botc_exec target =
          Botc_exec.ask pid "Who do you want to poison?" (alive_except state pid)
        in
        let%bind.Botc_exec () =
          Botc_exec.modify_state (fun s -> Game_state.set_poisoned s target)
        in
        Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
