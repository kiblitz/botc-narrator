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
          then
            (* Night 1: demon bluffs shown by demon_info helper in the script *)
            Botc_exec.return ()
          else (
            let candidates = Game_state.alive_ids state in
            let%bind.Botc_exec target =
              Botc_exec.ask pid "Who do you want to kill?" candidates
            in
            if Player_id.equal target pid
            then (
              (* Starpassing *)
              let minions =
                List.filter (Game_state.alive_players state) ~f:(fun p ->
                  is_minion (Player.character p))
              in
              match minions with
              | [] -> Botc_exec.modify_state (fun s -> Game_state.kill s pid)
              | _ ->
                let%bind.Botc_exec new_imp, _ = narrator_pick_from minions in
                Botc_exec.modify_state (fun s ->
                  let s = Game_state.kill s pid in
                  let players =
                    Map.update (Game_state.players s) (Player.id new_imp) ~f:(function
                      | None -> failwith "imp starpass: target missing"
                      | Some p -> { p with Player.character = to_display t })
                  in
                  Game_state.set_players s players))
            else (
              let%bind.Botc_exec st = Botc_exec.get_state in
              let is_protected =
                Option.equal Player_id.equal (Game_state.monk_protected st) (Some target)
              in
              let target_char =
                Player.character (Map.find_exn (Game_state.players st) target)
              in
              let is_soldier = String.equal (Char_display.id target_char) "soldier" in
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
