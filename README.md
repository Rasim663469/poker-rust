# Casino Royale - Commandes d’exécution

Ce projet contient plusieurs binaires Rust :

- `poker-rust` : application GUI (menu casino, Poker Solo/Online, Blackjack)
- `server` : serveur Poker Online (TCP)
- `client` : client CLI Poker Online (optionnel)

---

Ancienne version CLI et la version graphique de maintenant sont mélangés

## 1) Pré-requis

- Rust + Cargo installés
- Se placer dans le dossier du projet :
  ```bash
  cd poker-rust
- Docker
## 2) Lancer le docker
```
docker compose up
```

## 3) Lancer l’application GUI (local)
```bash
cargo run --bin poker-rust
```

## 4) Lancer le mode Poker Online (multijoueur)
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
