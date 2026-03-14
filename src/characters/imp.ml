open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "imp"
let name      = "Imp"
let kind      = Character_intf.Demon
let alignment = Character_intf.Evil

(** Character value used when a starpassed player becomes the new Imp.
    An inline anonymous module avoids any self-referential module issue. *)
let as_character : Character_intf.t =
  (module struct
    let id = "imp"  let name = "Imp"
    let kind = Character_intf.Demon  let alignment = Character_intf.Evil
  end : Character_intf.S)

let night_action ~player_id:pid ~night =
  Some (Character.if_alive pid begin
    let* ()    = Narrator_monad.wake pid in
    let* state = Narrator_monad.get_state in
    let* () =
      if night = 1 then
        (* Night 1: demon bluffs shown by demon_info helper in the script *)
        Narrator_monad.return ()
      else begin
        let candidates = Game_state.alive_ids state in
        let* target    = Narrator_monad.player_points pid candidates in
        if Player_id.equal target pid then begin
          (* Starpassing *)
          let minions =
            List.filter (Game_state.alive_players state)
              ~f:(fun p -> Character_intf.is_minion p.Player.character)
          in
          match minions with
          | [] -> Narrator_monad.modify_state (fun s -> Game_state.kill s pid)
          | _  ->
            let* (new_imp, _) = Character.narrator_pick_from minions in
            Narrator_monad.modify_state (fun s ->
              let s       = Game_state.kill s pid in
              let players =
                Map.update s.Game_state.players new_imp.Player.id ~f:(function
                  | None   -> failwith "imp starpass: target missing"
                  | Some p -> { p with Player.character = as_character })
              in
              { s with Game_state.players; imp_starpass = true })
        end else begin
          let* st = Narrator_monad.get_state in
          let is_protected =
            Option.equal Player_id.equal st.Game_state.monk_protected (Some target)
          in
          let target_char = (Map.find_exn st.Game_state.players target).Player.character in
          let is_soldier  = String.equal (Character_intf.id target_char) "soldier" in
          if is_protected || is_soldier then Narrator_monad.return ()
          else Narrator_monad.modify_state (fun s -> Game_state.kill s target)
        end
      end
    in
    Narrator_monad.sleep pid
  end)
