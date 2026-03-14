open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "investigator"
let name      = "Investigator"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night =
  if night <> 1 then None
  else Some (Character.if_alive pid begin
    let* state = Narrator_monad.get_state in
    let* ()    = Narrator_monad.wake pid in
    let* () =
      if Game_state.is_poisoned state pid then begin
        let ids = Game_state.alive_ids state in
        let* (p1, rest) = Character.narrator_pick_from ids in
        let* (p2, _)    = Character.narrator_pick_from rest in
        let minion_names =
          List.filter_map (Game_state.alive_players state) ~f:(fun p ->
            if Character_intf.is_minion p.Player.character
            then Some (Character_intf.name p.Player.character)
            else None)
        in
        let* char_name = Narrator_monad.narrator_pick minion_names in
        Narrator_monad.show_info pid (Narrator_monad.Two_players_one_char (p1, p2, char_name))
      end else begin
        let minions =
          List.filter (Game_state.alive_players state)
            ~f:(fun p -> Character_intf.is_minion p.Player.character)
        in
        let* (minion, _) = Character.narrator_pick_from minions in
        let other_ids    = Character.alive_except state minion.Player.id in
        let* (other, _)  = Character.narrator_pick_from other_ids in
        let* (swap, _)   = Character.narrator_pick_from [false; true] in
        let p1, p2       = if swap then (minion.Player.id, other) else (other, minion.Player.id) in
        Narrator_monad.show_info pid
          (Narrator_monad.Two_players_one_char (p1, p2, Character_intf.name minion.Player.character))
      end
    in
    Narrator_monad.sleep pid
  end)
