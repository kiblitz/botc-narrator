open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "chef"
    let name = "Chef"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night =
  if night <> 1
  then None
  else
    Some
      (if_alive
         pid
         (let%bind.Botc_exec state = Botc_exec.get_state
          and () = Botc_exec.wake pid in
          let%bind.Botc_exec () =
            if Game_state.is_poisoned state pid
            then (
              let%bind.Botc_exec ns =
                Botc_exec.narrator_pick "poisoned chef count" [ 0; 1; 2; 3 ] ~pick_count:1
              in
              let n = List.hd_exn ns in
              Botc_exec.tell pid [%string "%{n#Int} evil pairs"])
            else (
              let alive = Game_state.alive_ids state in
              let n = List.length alive in
              let count =
                List.counti alive ~f:(fun i pid ->
                  let next = List.nth_exn alive ((i + 1) mod n) in
                  Game_state.is_evil state pid && Game_state.is_evil state next)
              in
              Botc_exec.tell pid [%string "%{count#Int} evil pairs"])
          in
          Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
