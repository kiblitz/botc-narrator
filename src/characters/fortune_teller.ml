open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "fortune_teller"
    let name = "Fortune Teller"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night:_ =
  Some
    (if_alive
       pid
       (let%bind.Botc_exec state = Botc_exec.get_state
        and () = Botc_exec.wake pid in
        let candidates = Game_state.alive_ids state in
        let%bind.Botc_exec p1 = Botc_exec.ask pid "Choose a player" candidates in
        let%bind.Botc_exec p2 = Botc_exec.ask pid "Choose a player" candidates in
        let result =
          if Game_state.is_poisoned state pid
          then false
          else
            Kind.equal (Game_state.kind state p1) Kind.Demon
            || Kind.equal (Game_state.kind state p2) Kind.Demon
        in
        let%bind.Botc_exec () = Botc_exec.tell pid (if result then "Yes" else "No") in
        Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
