use crate::core::cards::{Carte, Paquet};
use crate::db::joueur_repo;
use crate::db::DbPool;
use crate::network::protocol::{ActionJoueur, MessageClient, MessageServeur};
use crate::network::{recv_json, send_json};
use std::cmp::Ordering;
use std::io;
use tokio::net::{TcpListener, TcpStream};

struct RemotePlayer {
    db_id: i32,
    nom: String,
    stream: TcpStream,
    jetons: u32,
    main: Vec<Carte>,
    couche: bool,
    mise_tour: u32,
}

pub async fn run_poker_server(addr: &str, pool: DbPool) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Serveur poker en ecoute sur {addr}");

    let (mut host_stream, host_addr) = listener.accept().await?;
    let (host_id, host_name, host_jetons) = authentifier_joueur(&mut host_stream, &pool).await?;
    println!("Hote connecte: {host_name} ({host_addr})");

    send_json(&mut host_stream, &MessageServeur::DemanderConfiguration).await?;
    let nb_joueurs_cfg = match recv_json::<MessageClient, _>(&mut host_stream).await? {
        MessageClient::Action(ActionJoueur::ConfigurerPartie { nb_joueurs, .. }) => {
            nb_joueurs.clamp(2, 6) as usize
        }
        _ => 2,
    };

    let nb_joueurs = nb_joueurs_cfg;
    println!("Configuration: {nb_joueurs} joueurs.");

    let mut joueurs = Vec::with_capacity(nb_joueurs);
    joueurs.push(RemotePlayer {
        db_id: host_id,
        nom: host_name,
        stream: host_stream,
        jetons: host_jetons,
        main: Vec::new(),
        couche: false,
        mise_tour: 0,
    });

    while joueurs.len() < nb_joueurs {
        let (mut stream, addr) = listener.accept().await?;
        let (db_id, pseudo, jetons) = authentifier_joueur(&mut stream, &pool).await?;
        println!("Joueur connecte: {pseudo} ({addr})");
        send_json(
            &mut stream,
            &MessageServeur::Bienvenue {
                message: format!("Bienvenue {pseudo}. En attente de la table..."),
            },
        )
        .await?;
        joueurs.push(RemotePlayer {
            db_id,
            nom: pseudo,
            stream,
            jetons,
            main: Vec::new(),
            couche: false,
            mise_tour: 0,
        });
    }

    broadcast(
        &mut joueurs,
        &MessageServeur::Bienvenue {
            message: "Tous les joueurs sont connectes. Debut de la session.".to_string(),
        },
    )
    .await?;

    let mut dealer_idx = 0usize;
    let small_blind = 10_u32;
    let big_blind = 20_u32;

    loop {
        let actifs = joueurs.iter().filter(|j| j.jetons > 0).count();
        if actifs < 2 {
            let gagnant = joueurs
                .iter()
                .max_by_key(|j| j.jetons)
                .map(|j| j.nom.clone())
                .unwrap_or_else(|| "Personne".to_string());
            broadcast(
                &mut joueurs,
                &MessageServeur::AnnonceAction {
                    nom: "Serveur".to_string(),
                    action: format!("Session terminee. Gagnant: {gagnant}"),
                },
            )
            .await?;
            break;
        }

        jouer_manche(
            &mut joueurs,
            &mut dealer_idx,
            small_blind,
            big_blind,
        )
        .await?;
    }

    for j in &joueurs {
        if let Err(e) = joueur_repo::maj_jetons(&pool, j.db_id, j.jetons as i32).await {
            eprintln!("Erreur maj jetons pour {}: {e}", j.nom);
        }
    }

    Ok(())
}

async fn jouer_manche(
    joueurs: &mut [RemotePlayer],
    dealer_idx: &mut usize,
    small_blind: u32,
    big_blind: u32,
) -> io::Result<()> {
    let mut paquet = Paquet::nouveau();
    paquet.melanger();
    let mut board: Vec<Carte> = Vec::new();
    let mut pot = 0_u32;

    for j in joueurs.iter_mut() {
        j.main.clear();
        j.couche = j.jetons == 0;
        j.mise_tour = 0;
    }

    let participants: Vec<usize> = joueurs
        .iter()
        .enumerate()
        .filter_map(|(i, j)| if j.jetons > 0 { Some(i) } else { None })
        .collect();
    if participants.len() < 2 {
        return Ok(());
    }

    while !participants.contains(dealer_idx) {
        *dealer_idx = (*dealer_idx + 1) % joueurs.len();
    }

    let sb_idx = next_participant(participants.as_slice(), *dealer_idx);
    let bb_idx = next_participant(participants.as_slice(), sb_idx);

    pot += prelever(joueurs, sb_idx, small_blind);
    pot += prelever(joueurs, bb_idx, big_blind);
    let mut mise_actuelle = joueurs[sb_idx].mise_tour.max(joueurs[bb_idx].mise_tour);

    for _ in 0..2 {
        for i in participants.iter().copied() {
            if let Some(c) = paquet.tirer_carte() {
                joueurs[i].main.push(c);
            }
        }
    }

    broadcast(
        joueurs,
        &MessageServeur::AnnonceAction {
            nom: "Serveur".to_string(),
            action: format!(
                "Nouvelle main. Donneur={}, SB={}, BB={}, Pot initial={}",
                joueurs[*dealer_idx].nom, joueurs[sb_idx].nom, joueurs[bb_idx].nom, pot
            ),
        },
    )
    .await?;

    for i in participants.iter().copied() {
        let cartes = joueurs[i].main.clone();
        send_json(&mut joueurs[i].stream, &MessageServeur::MesCartes { cartes }).await?;
    }
    broadcast(joueurs, &MessageServeur::MajTable { pot, cartes_communes: board.clone() }).await?;

    let start_preflop = next_participant(participants.as_slice(), bb_idx);
    tour_mises(
        joueurs,
        &participants,
        start_preflop,
        &mut mise_actuelle,
        &mut pot,
        &board,
        big_blind,
    )
    .await?;
    if actifs_non_couches(joueurs, &participants) <= 1 {
        payer_par_abandon(joueurs, &participants, pot).await?;
        *dealer_idx = next_participant(participants.as_slice(), *dealer_idx);
        return Ok(());
    }

    reset_mises(joueurs, &participants);
    mise_actuelle = 0;
    burn(&mut paquet);
    tirer_board(&mut paquet, &mut board, 3);
    broadcast(joueurs, &MessageServeur::MajTable { pot, cartes_communes: board.clone() }).await?;
    let start_postflop = next_participant(participants.as_slice(), *dealer_idx);
    tour_mises(
        joueurs,
        &participants,
        start_postflop,
        &mut mise_actuelle,
        &mut pot,
        &board,
        big_blind,
    )
    .await?;
    if actifs_non_couches(joueurs, &participants) <= 1 {
        payer_par_abandon(joueurs, &participants, pot).await?;
        *dealer_idx = next_participant(participants.as_slice(), *dealer_idx);
        return Ok(());
    }

    reset_mises(joueurs, &participants);
    mise_actuelle = 0;
    burn(&mut paquet);
    tirer_board(&mut paquet, &mut board, 1);
    broadcast(joueurs, &MessageServeur::MajTable { pot, cartes_communes: board.clone() }).await?;
    tour_mises(
        joueurs,
        &participants,
        start_postflop,
        &mut mise_actuelle,
        &mut pot,
        &board,
        big_blind,
    )
    .await?;
    if actifs_non_couches(joueurs, &participants) <= 1 {
        payer_par_abandon(joueurs, &participants, pot).await?;
        *dealer_idx = next_participant(participants.as_slice(), *dealer_idx);
        return Ok(());
    }

    reset_mises(joueurs, &participants);
    mise_actuelle = 0;
    burn(&mut paquet);
    tirer_board(&mut paquet, &mut board, 1);
    broadcast(joueurs, &MessageServeur::MajTable { pot, cartes_communes: board.clone() }).await?;
    tour_mises(
        joueurs,
        &participants,
        start_postflop,
        &mut mise_actuelle,
        &mut pot,
        &board,
        big_blind,
    )
    .await?;

    if actifs_non_couches(joueurs, &participants) <= 1 {
        payer_par_abandon(joueurs, &participants, pot).await?;
    } else {
        showdown(joueurs, &participants, &board, pot).await?;
    }

    *dealer_idx = next_participant(participants.as_slice(), *dealer_idx);
    Ok(())
}

async fn tour_mises(
    joueurs: &mut [RemotePlayer],
    participants: &[usize],
    start_idx: usize,
    mise_actuelle: &mut u32,
    pot: &mut u32,
    board: &[Carte],
    big_blind: u32,
) -> io::Result<()> {
    let mut besoin_action: Vec<bool> = vec![false; joueurs.len()];
    for i in participants.iter().copied() {
        besoin_action[i] = !joueurs[i].couche && joueurs[i].jetons > 0;
    }

    let mut idx = start_idx;
    loop {
        if actifs_non_couches(joueurs, participants) <= 1 {
            break;
        }
        if !participants.iter().any(|&i| besoin_action[i]) {
            break;
        }

        if !besoin_action[idx] || joueurs[idx].couche || joueurs[idx].jetons == 0 {
            idx = next_participant(participants, idx);
            continue;
        }

        let to_call = mise_actuelle.saturating_sub(joueurs[idx].mise_tour);
        let min_raise = if *mise_actuelle == 0 {
            big_blind
        } else {
            *mise_actuelle + big_blind
        };
        let max_total = joueurs[idx].mise_tour + joueurs[idx].jetons;
        let peut_relancer = max_total >= min_raise && joueurs[idx].jetons > to_call;

        send_json(
            &mut joueurs[idx].stream,
            &MessageServeur::DemanderAction {
                to_call,
                peut_relancer,
                jetons_restants: joueurs[idx].jetons,
            },
        )
        .await?;

        let action_msg = recv_json::<MessageClient, _>(&mut joueurs[idx].stream).await;
        let action = match action_msg {
            Ok(MessageClient::Action(a)) => a,
            _ => ActionJoueur::Fold,
        };

        match action {
            ActionJoueur::Fold => {
                joueurs[idx].couche = true;
                besoin_action[idx] = false;
                announce(joueurs, idx, "fold".to_string()).await?;
            }
            ActionJoueur::Check => {
                if to_call > 0 {
                    joueurs[idx].couche = true;
                    besoin_action[idx] = false;
                    announce(joueurs, idx, "fold (check invalide)".to_string()).await?;
                } else {
                    besoin_action[idx] = false;
                    announce(joueurs, idx, "check".to_string()).await?;
                }
            }
            ActionJoueur::Call => {
                let paye = joueurs[idx].jetons.min(to_call);
                joueurs[idx].jetons -= paye;
                joueurs[idx].mise_tour += paye;
                *pot += paye;
                besoin_action[idx] = false;
                announce(joueurs, idx, format!("call {paye}")).await?;
            }
            ActionJoueur::Raise(total) => {
                if !peut_relancer || total < min_raise || total > max_total {
                    let paye = joueurs[idx].jetons.min(to_call);
                    joueurs[idx].jetons -= paye;
                    joueurs[idx].mise_tour += paye;
                    *pot += paye;
                    besoin_action[idx] = false;
                    announce(joueurs, idx, format!("call {paye} (raise invalide)"))
                        .await?;
                } else {
                    let delta = total.saturating_sub(joueurs[idx].mise_tour);
                    joueurs[idx].jetons -= delta;
                    joueurs[idx].mise_tour = total;
                    *pot += delta;
                    *mise_actuelle = total;

                    for i in participants.iter().copied() {
                        besoin_action[i] = !joueurs[i].couche && joueurs[i].jetons > 0;
                    }
                    besoin_action[idx] = false;
                    announce(joueurs, idx, format!("raise {total}")).await?;
                }
            }
            ActionJoueur::ConfigurerPartie { .. } => {
                joueurs[idx].couche = true;
                besoin_action[idx] = false;
                announce(joueurs, idx, "fold".to_string()).await?;
            }
        }

        idx = next_participant(participants, idx);
        broadcast(
            joueurs,
            &MessageServeur::MajTable {
                pot: *pot,
                cartes_communes: board.to_vec(),
            },
        )
        .await?;
    }

    Ok(())
}

fn prelever(joueurs: &mut [RemotePlayer], idx: usize, montant: u32) -> u32 {
    let p = montant.min(joueurs[idx].jetons);
    joueurs[idx].jetons -= p;
    joueurs[idx].mise_tour += p;
    p
}

fn reset_mises(joueurs: &mut [RemotePlayer], participants: &[usize]) {
    for i in participants.iter().copied() {
        joueurs[i].mise_tour = 0;
    }
}

fn actifs_non_couches(joueurs: &[RemotePlayer], participants: &[usize]) -> usize {
    participants
        .iter()
        .filter(|&&i| !joueurs[i].couche)
        .count()
}

fn next_participant(participants: &[usize], from: usize) -> usize {
    let pos = participants.iter().position(|&p| p == from).unwrap_or(0);
    participants[(pos + 1) % participants.len()]
}

fn burn(paquet: &mut Paquet) {
    let _ = paquet.tirer_carte();
}

fn tirer_board(paquet: &mut Paquet, board: &mut Vec<Carte>, n: usize) {
    for _ in 0..n {
        if let Some(c) = paquet.tirer_carte() {
            board.push(c);
        }
    }
}

async fn payer_par_abandon(
    joueurs: &mut [RemotePlayer],
    participants: &[usize],
    pot: u32,
) -> io::Result<()> {
    if let Some(&winner) = participants.iter().find(|&&i| !joueurs[i].couche) {
        joueurs[winner].jetons += pot;
        broadcast(
            joueurs,
            &MessageServeur::AnnonceAction {
                nom: "Serveur".to_string(),
                action: format!("{} gagne {} (abandon)", joueurs[winner].nom, pot),
            },
        )
        .await?;
    }
    Ok(())
}

async fn showdown(
    joueurs: &mut [RemotePlayer],
    participants: &[usize],
    board: &[Carte],
    pot: u32,
) -> io::Result<()> {
    let mut evaluations: Vec<(usize, u8, Vec<u8>, String)> = Vec::new();
    for i in participants.iter().copied() {
        if joueurs[i].couche {
            continue;
        }
        let mut cartes = joueurs[i].main.clone();
        cartes.extend(board.iter().copied());
        let (rang, dep, label) = crate::games::poker::engine::evaluer_holdem_pour_gui(&cartes);
        evaluations.push((i, rang, dep, label));
    }

    evaluations.sort_by(|a, b| compare_eval((a.1, &a.2), (b.1, &b.2)).reverse());
    if evaluations.is_empty() {
        return Ok(());
    }

    let best = (evaluations[0].1, evaluations[0].2.clone());
    let gagnants: Vec<usize> = evaluations
        .iter()
        .filter(|(_, r, d, _)| *r == best.0 && *d == best.1)
        .map(|(i, _, _, _)| *i)
        .collect();

    let part = pot / gagnants.len() as u32;
    let reste = pot % gagnants.len() as u32;
    for (k, i) in gagnants.iter().enumerate() {
        joueurs[*i].jetons += part + if k == 0 { reste } else { 0 };
    }

    let noms = gagnants
        .iter()
        .map(|i| joueurs[*i].nom.clone())
        .collect::<Vec<_>>()
        .join(", ");
    broadcast(
        joueurs,
        &MessageServeur::AnnonceAction {
            nom: "Serveur".to_string(),
            action: format!("Showdown: {noms} remportent {pot}"),
        },
    )
    .await?;

    Ok(())
}

fn compare_eval(a: (u8, &[u8]), b: (u8, &[u8])) -> Ordering {
    a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1))
}

async fn authentifier_joueur(
    stream: &mut TcpStream,
    pool: &DbPool,
) -> io::Result<(i32, String, u32)> {
    loop {
        let msg: MessageClient = recv_json(stream).await?;
        match msg {
            MessageClient::Login { pseudo, mot_de_passe } => {
                match joueur_repo::authentifier(pool, &pseudo, &mot_de_passe).await {
                    Ok(Some(j)) => {
                        send_json(stream, &MessageServeur::AuthOk { jetons: j.jetons as u32 }).await?;
                        return Ok((j.id, j.pseudo, j.jetons as u32));
                    }
                    Ok(None) => {
                        send_json(stream, &MessageServeur::AuthEchec {
                            raison: "Pseudo ou mot de passe incorrect.".to_string(),
                        }).await?;
                    }
                    Err(e) => {
                        send_json(stream, &MessageServeur::AuthEchec { raison: e }).await?;
                    }
                }
            }
            MessageClient::Inscription { pseudo, mot_de_passe } => {
                match joueur_repo::inscrire(pool, &pseudo, &mot_de_passe).await {
                    Ok(j) => {
                        send_json(stream, &MessageServeur::AuthOk { jetons: j.jetons as u32 }).await?;
                        return Ok((j.id, j.pseudo, j.jetons as u32));
                    }
                    Err(e) => {
                        send_json(stream, &MessageServeur::AuthEchec { raison: e }).await?;
                    }
                }
            }
            _ => {
                send_json(stream, &MessageServeur::AuthEchec {
                    raison: "Message inattendu. Envoyez Login ou Inscription.".to_string(),
                }).await?;
            }
        }
    }
}

async fn announce(joueurs: &mut [RemotePlayer], idx: usize, action: String) -> io::Result<()> {
    let nom = joueurs[idx].nom.clone();
    broadcast(joueurs, &MessageServeur::AnnonceAction { nom, action }).await
}

async fn broadcast(joueurs: &mut [RemotePlayer], msg: &MessageServeur) -> io::Result<()> {
    for j in joueurs.iter_mut() {
        send_json(&mut j.stream, msg).await?;
    }
    Ok(())
}
