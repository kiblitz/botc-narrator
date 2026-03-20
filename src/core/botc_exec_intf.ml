open! Core

module type Engine_S = sig
  val wake : Player_id.t -> unit
  val sleep : Player_id.t -> unit
  val tell : Player_id.t -> string -> unit
  val ask : Player_id.t -> string -> Player_id.t list -> Player_id.t
  val narrator_pick : string -> 'a list -> 'a
  val log : string -> unit
end

module type Botc_exec = sig
  type ro
  type rw

  (** Free monad over narrator instructions, parameterized by a phantom permission
      type. [ro] computations only read game state; [rw] computations may also
      modify it. *)
  type ('a, 'perm) t

  val return : 'a -> ('a, _) t
  val bind : ('a, 'p) t -> f:('a -> ('b, 'p) t) -> ('b, 'p) t
  val map : ('a, 'p) t -> f:('a -> 'b) -> ('b, 'p) t
  val ( >>= ) : ('a, 'p) t -> ('a -> ('b, 'p) t) -> ('b, 'p) t
  val ( >>| ) : ('a, 'p) t -> ('a -> 'b) -> ('b, 'p) t

  module Let_syntax : sig
    val return : 'a -> ('a, _) t
    val ( >>= ) : ('a, 'p) t -> ('a -> ('b, 'p) t) -> ('b, 'p) t

    module Let_syntax : sig
      val return : 'a -> ('a, _) t
      val bind : ('a, 'p) t -> f:('a -> ('b, 'p) t) -> ('b, 'p) t
      val map : ('a, 'p) t -> f:('a -> 'b) -> ('b, 'p) t
      val both : ('a, 'p) t -> ('b, 'p) t -> ('a * 'b, 'p) t

      module Open_on_rhs : sig end
    end
  end

  val wake : Player_id.t -> (unit, _) t
  val sleep : Player_id.t -> (unit, _) t
  val tell : Player_id.t -> string -> (unit, _) t
  val ask : Player_id.t -> string -> Player_id.t list -> (Player_id.t, _) t
  val narrator_pick : string -> 'a list -> pick_count:int -> ('a list, _) t
  val log : string -> (unit, _) t
  val get_state : unit -> (Game_state.t, _) t
  val set_state : Game_state.t -> (unit, rw) t
  val modify_state : (Game_state.t -> Game_state.t) -> (unit, rw) t
  val when_ : bool -> (unit, 'p) t -> (unit, 'p) t
  val iter : 'a list -> f:('a -> (unit, 'p) t) -> (unit, 'p) t
  val as_rw : ('a, ro) t -> ('a, rw) t

  module type Engine_S = Engine_S

  val run : (module Engine_S) -> Game_state.t -> ('a, rw) t -> 'a * Game_state.t
end
