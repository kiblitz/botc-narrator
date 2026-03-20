open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "slayer"
    let name = "Slayer"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:_ ~night:_ = None

let day_action ~player_id:pid =
  Some
    (let%bind.Botc_exec state = Botc_exec.get_state () in
     if Game_state.has_used_day_ability state pid
     then Botc_exec.return ()
     else (
       let%bind.Botc_exec () =
         Botc_exec.modify_state (fun s -> Game_state.use_day_ability s pid)
       in
       let%bind.Botc_exec target =
         Botc_exec.ask pid "Who do you want to slay?" (Game_state.alive_ids state)
       in
       if Kind.equal (Game_state.kind state target) Kind.Demon
       then Botc_exec.modify_state (fun s -> Game_state.kill s target)
       else Botc_exec.return ()))
;;

let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
