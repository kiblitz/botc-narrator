open! Core
open Botc_narrator_lib
open Test_helpers

(* Poisoner targets Chef on night 1, giving wrong info.
   Slayer shoots a non-demon on day 1.
   Night 2: Poisoner retargets Fortune Teller, Monk protects Empath, Imp kills Chef.

   Lucius=Imp  Ivy=Poisoner  Troi=Empath  Gordon=Chef
   Zoltar=Fortune_teller  Friar=Monk  Buffy=Slayer *)

let players =
  mk_players
    [ "Lucius", (module Characters.Imp : Character_intf.S)
    ; "Ivy", (module Characters.Poisoner)
    ; "Troi", (module Characters.Empath)
    ; "Gordon", (module Characters.Chef)
    ; "Zoltar", (module Characters.Fortune_teller)
    ; "Friar", (module Characters.Monk)
    ; "Buffy", (module Characters.Slayer)
    ]
;;

let night_1 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:1 in
  state
  |> run ~action:(Narrator.minion_info ())
  |> run ~action:(Narrator.demon_info Trouble_brewing.all ())
  |> run
       ~responses:[ p "Gordon" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run ~action:(night_action Characters.Chef.night_action ~player_id:"Gordon")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Ivy"; p "Lucius" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let day_1 ?(silent = false) state =
  let run = run ~silent in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Ivy" ]
       ~action:(day_action Characters.Slayer.day_action ~player_id:"Buffy")
;;

let night_2 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:2 in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Zoltar" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run
       ~responses:[ p "Troi" ]
       ~action:(night_action Characters.Monk.night_action ~player_id:"Friar")
  |> run
       ~responses:[ p "Gordon" ]
       ~action:(night_action Characters.Imp.night_action ~player_id:"Lucius")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Lucius"; p "Lucius" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let init () = night_state players
let through_night_1 () = init () |> night_1 ~silent:true
let through_day_1 () = through_night_1 () |> day_1 ~silent:true

(* === Tests === *)

let%expect_test "night 1" =
  let state = init () |> night_1 in
  print_grimoire state;
  [%expect
    {|
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Evil: Lucius, Ivy
    narrator->Ivy(Poisoner): sleep
    narrator->Lucius(Imp): wake
    narrator->Lucius(Imp): Bluffs: Butler, Drunk, Investigator
    narrator->Lucius(Imp): sleep
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Who do you want to poison?
    Ivy(Poisoner)->narrator: Gordon(Chef)
    narrator->Ivy(Poisoner): sleep
    narrator->Gordon(Chef): wake
    narrator->Gordon(Chef): 0 evil pairs
    narrator->Gordon(Chef): sleep
    narrator->Troi(Empath): wake
    narrator->Troi(Empath): 1 evil neighbors
    narrator->Troi(Empath): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Ivy(Poisoner)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Lucius(Imp)
    narrator->Zoltar(Fortune Teller): Yes
    narrator->Zoltar(Fortune Teller): sleep
                                         Lucius(Imp)

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                              Troi(Empath)



                   Zoltar(Fortune Teller)          Gordon(Chef) [poisoned]
    |}]
;;

let%expect_test "day 1" =
  let state = through_night_1 () |> day_1 in
  print_grimoire state;
  [%expect
    {|
    narrator->Buffy(Slayer): Who do you want to slay?
    Buffy(Slayer)->narrator: Ivy(Poisoner)
                                         Lucius(Imp)

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                              Troi(Empath)



                   Zoltar(Fortune Teller)          Gordon(Chef) [poisoned]
    |}]
;;

let%expect_test "night 2" =
  let state = through_day_1 () |> night_2 in
  print_grimoire state;
  [%expect
    {|
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Who do you want to poison?
    Ivy(Poisoner)->narrator: Zoltar(Fortune Teller)
    narrator->Ivy(Poisoner): sleep
    narrator->Friar(Monk): wake
    narrator->Friar(Monk): Who do you want to protect?
    Friar(Monk)->narrator: Troi(Empath)
    narrator->Friar(Monk): sleep
    narrator->Lucius(Imp): wake
    narrator->Lucius(Imp): Who do you want to kill?
    Lucius(Imp)->narrator: Gordon(Chef)
    narrator->Lucius(Imp): sleep
    narrator->Troi(Empath): wake
    narrator->Troi(Empath): 1 evil neighbors
    narrator->Troi(Empath): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Lucius(Imp)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Lucius(Imp)
    narrator->Zoltar(Fortune Teller): No
    narrator->Zoltar(Fortune Teller): sleep
                                         Lucius(Imp)

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                              Troi(Empath)



              Zoltar(Fortune Teller) [poisoned]      Gordon(Chef) [dead]
    |}]
;;
