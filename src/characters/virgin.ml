open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "virgin"
    let name = "Virgin"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:_ ~night:_ = None
let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None

let on_nominated ~player_id:pid ~nominator =
  Some
    (let%bind.Botc_exec state = Botc_exec.get_state in
     if Game_state.has_used_day_ability state pid
     then Botc_exec.return ()
     else (
       let%bind.Botc_exec () =
         Botc_exec.modify_state (fun s -> Game_state.use_day_ability s pid)
       in
       if Kind.equal (Game_state.kind state nominator) Kind.Townsfolk
       then Botc_exec.modify_state (fun s -> Game_state.kill s nominator)
       else Botc_exec.return ()))
;;

let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
