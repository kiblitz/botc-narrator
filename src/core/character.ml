open! Core
include Character0

module Packed = struct
  type t =
    | T :
        { state : 'a
        ; character : (module Character_intf.S with type t = 'a)
        }
        -> t
end

module Make (C : Character_intf.Input_S) : Character_intf.Base_S = struct
  include C

  let narrator_pick = Botc_exec.narrator_pick

  let alive_except state self =
    Game_state.alive_ids state |> List.filter ~f:(fun id -> not (Player_id.equal id self))
  ;;

  let if_alive pid m =
    let%bind.Botc_exec state = Botc_exec.get_state () in
    Botc_exec.when_ (Game_state.is_alive state pid) m
  ;;
end
