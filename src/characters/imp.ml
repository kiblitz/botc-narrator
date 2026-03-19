open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "imp"
    let name = "Imp"
    let t = { kind = Kind.Demon; alignment = Alignment.Evil }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night =
  Some
    (if_alive
       pid
       (let%bind.Botc_exec () = Botc_exec.wake pid
        and state = Botc_exec.get_state in
        let%bind.Botc_exec () =
          if night = 1
          then Botc_exec.return ()
          else (
            let candidates = Game_state.alive_ids state in
            let%bind.Botc_exec target =
              Botc_exec.ask pid "Who do you want to kill?" candidates
            in
            if Player_id.equal target pid
            then (
              (* Starpassing *)
              let minion_ids =
                List.filter (Game_state.alive_ids state) ~f:(fun id ->
                  Kind.equal (Game_state.kind state id) Kind.Minion)
              in
              match minion_ids with
              | [] -> Botc_exec.modify_state (fun s -> Game_state.kill s pid)
              | _ ->
                let%bind.Botc_exec new_imps =
                  narrator_pick "imp starpass target" minion_ids ~pick_count:1
                in
                let new_imp_id = List.hd_exn new_imps in
                Botc_exec.modify_state (fun s ->
                  let s = Game_state.kill s pid in
                  Game_state.transform_into
                    s
                    new_imp_id
                    ~character_id:id
                    ~character_name:name
                    ~kind:(kind t)
                    ~alignment:(alignment t)))
            else (
              let%bind.Botc_exec st = Botc_exec.get_state in
              let is_protected = Game_state.monk_protected st target in
              let is_soldier =
                String.equal (Game_state.character_id st target) "soldier"
              in
              if is_protected || is_soldier
              then Botc_exec.return ()
              else Botc_exec.modify_state (fun s -> Game_state.kill s target)))
        in
        Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
