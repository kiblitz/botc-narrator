open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "ravenkeeper"
let name      = "Ravenkeeper"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night =
  if night = 1 then None
  else Some begin
    let* state = Narrator_monad.get_state in
    let died_tonight =
      List.mem state.Game_state.night_deaths pid ~equal:Player_id.equal
    in
    if not died_tonight then Narrator_monad.return ()
    else begin
      let* ()     = Narrator_monad.wake pid in
      let* target = Narrator_monad.player_points pid (Game_state.alive_ids state) in
      let* () =
        if Game_state.is_poisoned state pid then begin
          let all_names =
            List.map (Game_state.seated_players state)
              ~f:(fun p -> Character_intf.name p.Player.character)
          in
          let* char_name = Narrator_monad.narrator_pick all_names in
          Narrator_monad.show_info pid (Narrator_monad.Character_revealed char_name)
        end else begin
          let char = (Map.find_exn state.Game_state.players target).Player.character in
          Narrator_monad.show_info pid (Narrator_monad.Character_revealed (Character_intf.name char))
        end
      in
      Narrator_monad.sleep pid
    end
  end
