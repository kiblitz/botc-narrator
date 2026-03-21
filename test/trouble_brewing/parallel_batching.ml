open! Core
open Botc_narrator_lib
open Test_helpers

(* Test that run_night batches consecutive read-only actions in parallel.

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

let%expect_test "night 1: ro actions batched in parallel after rw poisoner" =
  let state = night_state players in
  let state =
    run
      ~responses:[ p "Carol"; p "Alice"; p "Dave" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  print_grimoire state;
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
    [log] [parallel 0]
    narrator->Frank(Washerwoman): wake
    narrator->Frank(Washerwoman): Alice and Carol — one is the Empath
    narrator->Frank(Washerwoman): sleep
    [log] [parallel 1]
    narrator->Dave(Chef): wake
    narrator->Dave(Chef): 1 evil pairs
    narrator->Dave(Chef): sleep
    [log] [parallel 2]
    narrator->Carol(Empath): wake
    narrator->Carol(Empath): 0 evil neighbors
    narrator->Carol(Empath): sleep
    [log] [parallel 3]
    narrator->Eve(Fortune Teller): wake
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Alice(Imp)
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Dave(Chef)
    narrator->Eve(Fortune Teller): Yes
    narrator->Eve(Fortune Teller): sleep
                                         Alice(Imp)


    Frank(Washerwoman)                                                   Bob(Poisoner)





    Eve(Fortune Teller)                                            Carol(Empath) [poisoned]


                                         Dave(Chef)
    |}]
;;

let%expect_test "night 2: rw actions are sync points between ro batches" =
  let state = night_state players in
  (* Run night 1 silently *)
  let state =
    run
      ~silent:true
      ~responses:[ p "Carol"; p "Alice"; p "Dave" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  let state = Game_state.next_phase state |> Game_state.next_phase in
  (* Night 2: order is Poisoner(rw), Monk(n/a), Imp(rw), Ravenkeeper(n/a),
     Undertaker(n/a), Spy(n/a), Empath(ro), Fortune_teller(ro), Butler(n/a)
     So: Poisoner(rw) -> Imp(rw) -> Empath+Fortune_teller(ro batch) *)
  let state =
    run
      ~responses:[ p "Dave"; p "Dave"; p "Alice"; p "Bob" ]
      ~action:(Narrator.run_night (module Trouble_brewing) ())
      state
  in
  print_grimoire state;
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
    [log] [parallel 0]
    narrator->Carol(Empath): wake
    narrator->Carol(Empath): 1 evil neighbors
    narrator->Carol(Empath): sleep
    [log] [parallel 1]
    narrator->Eve(Fortune Teller): wake
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Alice(Imp)
    narrator->Eve(Fortune Teller): Choose a player
    Eve(Fortune Teller)->narrator: Bob(Poisoner)
    narrator->Eve(Fortune Teller): Yes
    narrator->Eve(Fortune Teller): sleep
                                         Alice(Imp)


    Frank(Washerwoman)                                                   Bob(Poisoner)





    Eve(Fortune Teller)                                                  Carol(Empath)


                                 Dave(Chef) [dead, poisoned]
    |}]
;;
