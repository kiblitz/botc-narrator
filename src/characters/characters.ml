open! Core

let ( let* ) = Narrator_monad.( let* )
let ( >>    ) = Narrator_monad.( >>  )

module Washerwoman    = Washerwoman
module Librarian      = Librarian
module Investigator   = Investigator
module Chef           = Chef
module Empath         = Empath
module Fortune_teller = Fortune_teller
module Undertaker     = Undertaker
module Monk           = Monk
module Ravenkeeper    = Ravenkeeper
module Virgin         = Virgin
module Slayer         = Slayer
module Soldier        = Soldier
module Mayor          = Mayor
module Butler         = Butler
module Drunk          = Drunk
module Recluse        = Recluse
module Saint          = Saint
module Poisoner       = Poisoner
module Spy            = Spy
module Scarlet_woman  = Scarlet_woman
module Baron          = Baron
module Imp            = Imp

(** All TB characters as first-class modules. *)
let all : Character_intf.t list = [
  (module Washerwoman);
  (module Librarian);
  (module Investigator);
  (module Chef);
  (module Empath);
  (module Fortune_teller);
  (module Undertaker);
  (module Monk);
  (module Ravenkeeper);
  (module Virgin);
  (module Slayer);
  (module Soldier);
  (module Mayor);
  (module Butler);
  (module Drunk);
  (module Recluse);
  (module Saint);
  (module Poisoner);
  (module Spy);
  (module Scarlet_woman);
  (module Baron);
  (module Imp);
]

(** Night 1: all alive Minions wake and see the full evil team. *)
let minion_info () =
  let* state  = Narrator_monad.get_state in
  let minions = List.filter (Game_state.alive_players state)
    ~f:(fun p -> Character_intf.is_minion p.Player.character)
  in
  if List.is_empty minions then Narrator_monad.return ()
  else begin
    let evil_ids =
      List.filter_map (Game_state.alive_players state) ~f:(fun p ->
        if Character_intf.is_evil p.Player.character then Some p.Player.id else None)
    in
    let* () = List.fold minions ~init:(Narrator_monad.return ())
                ~f:(fun acc p -> acc >> Narrator_monad.wake p.Player.id) in
    let* () = List.fold minions ~init:(Narrator_monad.return ())
                ~f:(fun acc p -> acc >> Narrator_monad.show_info p.Player.id (Narrator_monad.Evil_players evil_ids)) in
    List.fold minions ~init:(Narrator_monad.return ())
      ~f:(fun acc p -> acc >> Narrator_monad.sleep p.Player.id)
  end

(** Night 1: Demon wakes and sees three not-in-play good character names. *)
let demon_info () =
  let* state = Narrator_monad.get_state in
  let demon  = List.find (Game_state.alive_players state)
    ~f:(fun p -> Character_intf.is_demon p.Player.character)
  in
  match demon with
  | None -> Narrator_monad.return ()
  | Some d ->
    let* ()         = Narrator_monad.wake d.Player.id in
    let not_in_play = Game_state.good_char_names_not_in_play state all in
    let count       = min 3 (List.length not_in_play) in
    let* bluffs     = Character.pick_n count not_in_play in
    let* ()         = Narrator_monad.show_info d.Player.id (Narrator_monad.Demon_bluffs bluffs) in
    Narrator_monad.sleep d.Player.id
