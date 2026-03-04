# Casino Rust - Commandes d’exécution

Ce projet contient plusieurs binaires Rust :

- `poker-rust` : application GUI (menu casino, Poker Solo/Online, Blackjack)
- `server` : serveur Poker Online (TCP)
- `client` : client CLI Poker Online (optionnel)

---

## 1) Pré-requis

- Rust + Cargo installés
- Se placer dans le dossier du projet :
  ```bash
  cd poker-rust
## 2) Lancer l’application GUI (local)
```bash
cargo run --bin poker-rust
```

## 3) Lancer le mode Poker Online (multijoueur)
Étape A - Démarrer le serveur
Dans un premier terminal:
```bash
cargo run --bin server
```
Étape B - Lancer les clients GUI
Dans un second terminal (et plus si besoin) :
```bash
cargo run --bin poker-rust
```
