(lang dune 3.18)

(name botc_narrator)

(generate_opam_files true)

(source
 (github username/reponame))

(authors "Author Name <author@example.com>")

(maintainers "Maintainer Name <maintainer@example.com>")

(license LICENSE)

(package
 (name botc_narrator)
 (synopsis "Blood on the Clocktower narrator library")
 (description "A library for narrating Blood on the Clocktower, starting with Trouble Brewing")
 (depends
  ocaml
  (core (>= v0.16))
  (ppx_jane (>= v0.16))
  (ppx_sexp_conv (>= v0.16))
  (ppx_compare (>= v0.16))
  (ppx_enumerate (>= v0.16))))
