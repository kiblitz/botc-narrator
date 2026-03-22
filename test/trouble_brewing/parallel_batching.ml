open! Core
open Botc_narrator_lib
open Test_helpers

(* Test that run_night batches consecutive read-only night actions in parallel
   and treats read-write actions as sequential sync points.

   Alice=Imp  Bob=Poisoner  Carol=Empath  Dave=Chef
   Eve=Fortune_teller  Frank=Washerwoman *)

let players =
  mk_players
    [ "Alice", (module Characters.Imp : Character_intf.S)
    ; "Bob", (module Characters.Poisoner)
    ; "Carol", (module Characters.Empath)
    ; "Dave", (module Characters.Chef)
    ; "Eve", (module Characters.Fortune_teller)
    ; "Frank", (module Characters.Washerwoman)
    ]
;;

let%expect_test "night 1: ro actions batched after rw poisoner" =
  let state = night_state players in
  let _state =
    run
      ~responses:[ p "Carol"; p "Alice"; p "Dave" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  [%expect
    {|
    narrator->Bob(Poisoner): wake
    narrator->Bob(Poisoner): Evil: Alice, Bob
    narrator->Bob(Poisoner): sleep
    narrator->Alice(Imp): wake
    narrator->Alice(Imp): Bluffs: Butler, Drunk, Investigator
    narrator->Alice(Imp): sleep
    narrator->Bob(Poisoner): wake
    narrator->Bob(Poisoner): Who do you want to poison?
    Bob(Poisoner)->narrator: Carol(Empath)
    narrator->Bob(Poisoner): sleep
    === parallel (4) ===
    narrator->Frank(Washerwoman): wake
    narrator->Frank(Washerwoman): Alice and Carol — one is the Empath
    narrator->Frank(Washerwoman): sleep
    narrator->Dave(Chef): wake
    narrator->Dave(Chef): 1 evil pairs
    narrator->Dave(Chef): sleep
    narrator->Carol(Empath): wake
    narrator->Carol(Empath): 0 evil neighbors
    narrator->Carol(Empath): sleep
    narrator->Eve(Fortune Teller): wake
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Alice(Imp)
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Dave(Chef)
    narrator->Eve(Fortune Teller): Yes
    narrator->Eve(Fortune Teller): sleep
    === end parallel ===
    |}]
;;

let%expect_test "night 2: rw sync points separate ro batches" =
  let state = night_state players in
  let state =
    run
      ~silent:true
      ~responses:[ p "Carol"; p "Alice"; p "Dave" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  let state = Game_state.next_phase state |> Game_state.next_phase in
  (* Night 2 order: Poisoner(rw), Imp(rw), Empath(ro), Fortune_teller(ro)
     So: two rw sync points, then a parallel ro batch *)
  let _state =
    run
      ~responses:[ p "Dave"; p "Dave"; p "Alice"; p "Bob" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  [%expect
    {|
    narrator->Bob(Poisoner): wake
    narrator->Bob(Poisoner): Who do you want to poison?
    Bob(Poisoner)->narrator: Dave(Chef)
    narrator->Bob(Poisoner): sleep
    narrator->Alice(Imp): wake
    narrator->Alice(Imp): Who do you want to kill?
    Alice(Imp)->narrator: Dave(Chef)
    narrator->Alice(Imp): sleep
    === parallel (2) ===
    narrator->Carol(Empath): wake
    narrator->Carol(Empath): 1 evil neighbors
    narrator->Carol(Empath): sleep
    narrator->Eve(Fortune Teller): wake
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Alice(Imp)
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Bob(Poisoner)
    narrator->Eve(Fortune Teller): Yes
    narrator->Eve(Fortune Teller): sleep
    === end parallel ===
    |}]
;;
