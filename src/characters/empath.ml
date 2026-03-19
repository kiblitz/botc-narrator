open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "empath"
    let name = "Empath"
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
        let%bind.Botc_exec () =
          if Game_state.is_poisoned state pid
          then (
            let%bind.Botc_exec ns =
              Botc_exec.narrator_pick "poisoned empath count" [ 0; 1; 2 ] ~pick_count:1
            in
            let n = List.hd_exn ns in
            Botc_exec.tell pid [%string "%{n#Int} evil neighbors"])
          else (
            let alive = Game_state.alive_ids state in
            let n = List.length alive in
            let idx =
              List.findi alive ~f:(fun _ id -> Player_id.equal id pid)
              |> Option.value_exn ~message:"empath: player not found"
              |> fst
            in
            let left = List.nth_exn alive ((idx - 1 + n) mod n) in
            let right = List.nth_exn alive ((idx + 1) mod n) in
            let count =
              (if Game_state.is_evil state left then 1 else 0)
              + if Game_state.is_evil state right then 1 else 0
            in
            Botc_exec.tell pid [%string "%{count#Int} evil neighbors"])
        in
        Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
