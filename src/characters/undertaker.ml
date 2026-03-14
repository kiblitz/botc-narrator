open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "undertaker"
let name      = "Undertaker"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night =
  if night = 1 then None
  else Some (Character.if_alive pid begin
    let* state = Narrator_monad.get_state in
    match state.Game_state.last_execution with
    | None -> Narrator_monad.return ()
    | Some exec_id ->
      let* () = Narrator_monad.wake pid in
      let* () =
        if Game_state.is_poisoned state pid then begin
          let all_names =
            List.map (Game_state.seated_players state)
              ~f:(fun p -> Character_intf.name p.Player.character)
          in
          let* char_name = Narrator_monad.narrator_pick all_names in
          Narrator_monad.show_info pid (Narrator_monad.Character_revealed char_name)
        end else begin
          let exec_char =
            (Map.find_exn state.Game_state.players exec_id).Player.character
          in
          Narrator_monad.show_info pid
            (Narrator_monad.Character_revealed (Character_intf.name exec_char))
        end
      in
      Narrator_monad.sleep pid
  end)
