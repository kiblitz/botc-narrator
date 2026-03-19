open! Core
open Botc_narrator_lib
open Test_helpers

(* A Townsfolk nominates the Virgin on day 1 and dies.

   Diablo=Imp  Ivy=Poisoner  Maiden=Virgin  Gordon=Chef
   Troi=Empath  Zoltar=Fortune_teller  Buffy=Slayer *)

let players =
  mk_players
    [ "Diablo", (module Characters.Imp : Character_intf.S)
    ; "Ivy", (module Characters.Poisoner)
    ; "Maiden", (module Characters.Virgin)
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
       ~responses:[ p "Troi" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run ~action:(night_action Characters.Chef.night_action ~player_id:"Gordon")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Gordon"; p "Buffy" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let init () = night_state players

let%expect_test "day 1: townsfolk nominates virgin and dies" =
  let state =
    init ()
    |> night_1 ~silent:true
    |> fun s ->
    Game_state.next_phase s
    |> run
         ~action:(Option.value_exn
            (Characters.Virgin.on_nominated
               ~player_id:(p "Maiden")
               ~nominator:(p "Gordon")))
  in
  print_grimoire state;
  [%expect
    {|
                                               Diablo(Imp)

                Buffy(Slayer)                                               Ivy(Poisoner)




    Zoltar(Fortune Teller)                                                        Maiden(Virgin)



                         Troi(Empath) [poisoned]           Gordon(Chef) [dead]
    |}]
;;
