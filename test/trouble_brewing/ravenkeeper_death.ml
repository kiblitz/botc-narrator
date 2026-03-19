open! Core
open Botc_narrator_lib
open Test_helpers

(* Imp kills Ravenkeeper on night 2. Ravenkeeper wakes and learns a character.

   Diablo=Imp  Ivy=Poisoner  Raven=Ravenkeeper  Gordon=Chef
   Troi=Empath  Zoltar=Fortune_teller  Buffy=Slayer *)

let players =
  mk_players
    [ "Diablo", (module Characters.Imp : Character_intf.S)
    ; "Ivy", (module Characters.Poisoner)
    ; "Raven", (module Characters.Ravenkeeper)
    ; "Gordon", (module Characters.Chef)
    ; "Troi", (module Characters.Empath)
    ; "Zoltar", (module Characters.Fortune_teller)
    ; "Buffy", (module Characters.Slayer)
    ]
;;

let run = run ~players

let night_1 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:1 in
  state
  |> run ~action:(Narrator.minion_info ())
  |> run ~action:(Narrator.demon_info Trouble_brewing.all ())
  |> run
       ~responses:[ p "Buffy" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run ~action:(night_action Characters.Chef.night_action ~player_id:"Gordon")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Troi"; p "Gordon" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let night_2 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:2 in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Gordon" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run
       ~responses:[ p "Raven" ]
       ~action:(night_action Characters.Imp.night_action ~player_id:"Diablo")
  |> run
       ~responses:[ p "Ivy" ]
       ~action:(night_action Characters.Ravenkeeper.night_action ~player_id:"Raven")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Diablo"; p "Ivy" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let init () = night_state players

let%expect_test "night 2: ravenkeeper dies, learns a role" =
  let state =
    init () |> night_1 ~silent:true |> fun s -> Game_state.next_phase s |> night_2
  in
  print_grimoire state;
  [%expect
    {|
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Who do you want to poison?
    Ivy(Poisoner)->narrator: Gordon(Chef)
    narrator->Ivy(Poisoner): sleep
    narrator->Diablo(Imp): wake
    narrator->Diablo(Imp): Who do you want to kill?
    Diablo(Imp)->narrator: Raven(Ravenkeeper)
    narrator->Diablo(Imp): sleep
    narrator->Raven(Ravenkeeper): wake
    narrator->Raven(Ravenkeeper): Who do you want to learn about?
    Raven(Ravenkeeper)->narrator: Ivy(Poisoner)
    narrator->Raven(Ravenkeeper): Poisoner
    narrator->Raven(Ravenkeeper): sleep
    narrator->Troi(Empath): wake
    narrator->Troi(Empath): 0 evil neighbors
    narrator->Troi(Empath): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Diablo(Imp)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Ivy(Poisoner)
    narrator->Zoltar(Fortune Teller): Yes
    narrator->Zoltar(Fortune Teller): sleep
                                               Diablo(Imp)

                Buffy(Slayer)                                               Ivy(Poisoner)




    Zoltar(Fortune Teller)                                                   Raven(Ravenkeeper) [dead]



                              Troi(Empath)               Gordon(Chef) [poisoned]
    |}]
;;
