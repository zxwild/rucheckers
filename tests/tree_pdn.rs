//! Дерево партии, нотация и файлы PDN.

mod common;

use common::{mv, play, play_all, pos};
use rucheckers::board::{Color, Position};
use rucheckers::moves::legal_moves;
use rucheckers::notation::format_move;
use rucheckers::pdn::{
    from_bytes, from_pdn, result_token, standard_tags, to_pdn, MoveProblem, PdnError, GAME_TYPE,
    MAX_FILE_SIZE,
};
use rucheckers::rng::Rng;
use rucheckers::rules::{DrawReason, Outcome};
use rucheckers::tree::{GameTree, NodeId, TreeError};

// ---------------------------------------------------------------------------
// Дерево партии
// ---------------------------------------------------------------------------

/// Основная линия из четырёх полуходов; возвращает идентификаторы узлов.
fn sample_line(tree: &mut GameTree) -> Vec<NodeId> {
    ["c3-d4", "f6-e5", "d4:f6", "g7:e5"]
        .iter()
        .map(|text| play(tree, text))
        .collect()
}

#[test]
fn moves_form_a_line_with_positions_and_ply_numbers() {
    let mut tree = GameTree::new();
    assert!(tree.is_empty());
    let ids = sample_line(&mut tree);
    assert_eq!(tree.len(), 5);
    assert_eq!(tree.current(), ids[3]);
    assert_eq!(
        tree.path_to(ids[3]),
        [GameTree::ROOT, ids[0], ids[1], ids[2], ids[3]]
    );
    for (i, &id) in ids.iter().enumerate() {
        let node = tree.get(id).unwrap();
        assert_eq!(node.ply(), i as u32 + 1);
        let expected_side = if i % 2 == 0 { Color::Black } else { Color::White };
        assert_eq!(node.position().side, expected_side);
        assert_eq!(node.outcome(), None);
    }
    assert_eq!(tree.current_node().position().count(Color::White), 11);
    assert_eq!(tree.current_node().position().count(Color::Black), 11);
    assert_eq!(tree.root().position(), &Position::initial());
    assert_eq!(tree.main_line_end(), ids[3]);
}

#[test]
fn a_different_move_creates_a_variation_and_keeps_the_old_continuation() {
    let mut tree = GameTree::new();
    let ids = sample_line(&mut tree);
    // Возвращаемся к позиции после первого хода и отвечаем иначе.
    assert!(tree.go_to(ids[0]));
    let alt = play(&mut tree, "b6-a5");
    assert_ne!(alt, ids[1]);
    let parent = tree.get(ids[0]).unwrap();
    assert_eq!(
        parent.children(),
        [ids[1], alt],
        "прежнее продолжение осталось первым"
    );
    assert_eq!(tree.len(), 6);
    // Прежнее продолжение сохранено целиком.
    assert_eq!(tree.get(ids[1]).unwrap().children(), [ids[2]]);
    assert_eq!(tree.get(ids[2]).unwrap().children(), [ids[3]]);
    assert_eq!(tree.main_line_end(), ids[3], "основная линия не изменилась");
    // Тот же ход повторно варианта не создаёт.
    tree.go_to(ids[0]);
    assert_eq!(play(&mut tree, "b6-a5"), alt);
    assert_eq!(tree.len(), 6);
    // Третий вариант в той же позиции и вариант в самом начале партии.
    tree.go_to(ids[0]);
    let third = play(&mut tree, "h6-g5");
    assert_eq!(tree.siblings(third), [ids[1], alt, third]);
    tree.go_to(GameTree::ROOT);
    let first_alt = play(&mut tree, "e3-f4");
    assert_eq!(tree.root().children(), [ids[0], first_alt]);
    assert_eq!(tree.siblings(first_alt), [ids[0], first_alt]);
    assert_eq!(tree.siblings(GameTree::ROOT), [] as [NodeId; 0]);
}

#[test]
fn navigation_follows_the_active_branch() {
    let mut tree = GameTree::new();
    let ids = sample_line(&mut tree);
    tree.go_to(ids[0]);
    let alt = play(&mut tree, "b6-a5");
    let alt_reply = play(&mut tree, "d4-c5");

    // Назад и вперёд: «вперёд» идёт по той ветке, по которой пришли.
    assert!(tree.back());
    assert!(tree.back());
    assert_eq!(tree.current(), ids[0]);
    assert!(tree.forward());
    assert_eq!(tree.current(), alt);
    assert!(tree.go_end());
    assert_eq!(tree.current(), alt_reply);
    assert!(!tree.forward(), "дальше ходов нет");
    assert!(!tree.go_end());
    assert_eq!(tree.active_line(), [GameTree::ROOT, ids[0], alt, alt_reply]);

    // Переключение между вариантами.
    tree.go_to(alt);
    assert!(tree.switch_variation(1));
    assert_eq!(
        tree.current(),
        ids[1],
        "по кругу: за последним вариантом идёт первый"
    );
    assert!(tree.switch_variation(-1));
    assert_eq!(tree.current(), alt);
    assert!(tree.switch_variation(-1));
    assert_eq!(tree.current(), ids[1]);
    // Теперь активна основная линия.
    assert_eq!(
        tree.active_line(),
        [GameTree::ROOT, ids[0], ids[1], ids[2], ids[3]]
    );
    tree.go_to(ids[0]);
    assert!(!tree.switch_variation(1), "у первого хода нет вариантов");
    assert_eq!(
        tree.active_line().last(),
        Some(&ids[3]),
        "переход назад не меняет активную ветку"
    );

    // К началу и переход к произвольному узлу.
    assert!(tree.go_start());
    assert_eq!(tree.current(), GameTree::ROOT);
    assert!(!tree.go_start());
    assert!(!tree.back());
    assert!(!tree.switch_variation(1));
    assert!(tree.go_to(alt_reply));
    assert_eq!(tree.active_line(), [GameTree::ROOT, ids[0], alt, alt_reply]);
    tree.go_start();
    assert!(tree.go_end());
    assert_eq!(tree.current(), alt_reply, "конец активной, а не основной ветки");
}

#[test]
fn play_continues_from_the_middle_of_the_game() {
    let mut tree = GameTree::new();
    let ids = sample_line(&mut tree);
    // Продолжение с середины записанным ходом: идём по существующей ветке.
    tree.go_to(ids[0]);
    assert_eq!(play(&mut tree, "f6-e5"), ids[1]);
    assert_eq!(tree.len(), 5);
    // Продолжение новым ходом: новая ветка, счётчик полуходов считается от корня.
    tree.go_to(ids[0]);
    let alt = play(&mut tree, "b6-a5");
    assert_eq!(tree.get(alt).unwrap().ply(), 2);
    assert_eq!(tree.get(alt).unwrap().parent(), Some(ids[0]));
    let next = play(&mut tree, "d4-c5");
    assert_eq!(tree.path_to(next), [GameTree::ROOT, ids[0], alt, next]);
    assert_eq!(tree.len(), 7);
}

#[test]
fn illegal_moves_are_rejected_and_the_tree_is_unchanged() {
    let mut tree = GameTree::new();
    let ids = sample_line(&mut tree);
    let before = tree.len();
    // Ход из другой позиции (сейчас очередь белых, взятия нет).
    let foreign = mv(&Position::initial(), "c3-d4");
    assert_eq!(tree.play(&foreign), Err(TreeError::IllegalMove));
    // Тихий ход при обязательном взятии.
    let quiet = mv(&pos("W:Wa1:Bh8"), "a1-b2");
    assert_eq!(tree.add_move(ids[1], &quiet), Err(TreeError::IllegalMove));
    assert_eq!(tree.add_move(12345, &quiet), Err(TreeError::NoSuchNode));
    assert_eq!(tree.len(), before);
    assert_eq!(tree.current(), ids[3]);
}

#[test]
fn no_moves_can_be_added_after_the_game_is_over() {
    let mut tree = GameTree::from_position(pos("W:Wc3:Bd4"));
    let end = play(&mut tree, "c3:e5");
    assert!(tree.current_node().outcome().is_some());
    let any = mv(&Position::initial(), "c3-d4");
    assert_eq!(tree.add_move(end, &any), Err(TreeError::GameOver));

    // После ничьей по правилам ходы тоже не принимаются.
    let mut tree = GameTree::from_position(pos("W:WKg1,a1,c1,e1:BKb8,d8,f8,h8"));
    for _ in 0..2 {
        play_all(&mut tree, &["g1-h2", "b8-a7", "h2-g1", "a7-b8"]);
    }
    assert_eq!(
        tree.current_node().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
    let next = mv(tree.current_node().position(), "g1-h2");
    assert_eq!(tree.play(&next), Err(TreeError::GameOver));
    // Но из более ранней позиции той же ветки партию можно продолжить иначе.
    assert!(tree.back());
    let other = play(&mut tree, "a7-b6");
    assert_eq!(tree.get(other).unwrap().outcome(), None);
}

#[test]
fn draw_counters_follow_the_branch_leading_to_the_node() {
    // Основная ветка: дамки ходят туда-обратно, счётчик правила 3 растёт.
    let mut tree = GameTree::from_position(pos("W:WKg1,a1,c1,e1:BKb8,d8,f8,h8"));
    play_all(&mut tree, &["g1-h2", "b8-a7", "h2-g1"]);
    let node = tree.current();
    assert_eq!(tree.current_node().state().counters.kings_only, 3);
    let main = play(&mut tree, "a7-b8");
    assert_eq!(tree.get(main).unwrap().state().counters.kings_only, 4);
    assert_eq!(tree.get(main).unwrap().occurrences(), 2);
    assert_eq!(tree.history_before(main).len(), 4);
    // Вариант: вместо хода дамкой — ход простой; счётчик на этой ветке обнулён.
    tree.go_to(node);
    let side = play(&mut tree, "h8-g7");
    assert_eq!(tree.get(side).unwrap().state().counters.kings_only, 0);
    assert_eq!(tree.get(side).unwrap().occurrences(), 1);
    assert!(
        tree.history_before(side).is_empty(),
        "после хода простой повторения невозможны"
    );
    // Основная ветка при этом не изменилась.
    assert_eq!(tree.get(main).unwrap().state().counters.kings_only, 4);
}

// ---------------------------------------------------------------------------
// Случайные деревья
// ---------------------------------------------------------------------------

/// Случайное дерево: в основном линии продолжаются, изредка от случайного узла
/// ответвляется вариант.
fn random_tree(rng: &mut Rng, start: Position, steps: usize) -> GameTree {
    let mut tree = GameTree::from_position(start);
    let mut last = GameTree::ROOT;
    for _ in 0..steps {
        let from = if rng.chance(0.8) {
            last
        } else {
            rng.below(tree.len())
        };
        let node = tree.get(from).unwrap();
        if node.outcome().is_some() {
            last = rng.below(tree.len());
            continue;
        }
        let moves = legal_moves(node.position());
        let m = *rng.pick(&moves).unwrap();
        last = tree.add_move(from, &m).unwrap();
    }
    tree
}

fn max_children(tree: &GameTree) -> usize {
    (0..tree.len())
        .map(|id| tree.get(id).unwrap().children().len())
        .max()
        .unwrap_or(0)
}

#[test]
fn same_structure_detects_differences() {
    let mut rng = Rng::new(5);
    let tree = random_tree(&mut rng, Position::initial(), 120);
    assert!(tree.same_structure(&tree.clone()));
    // Лишний ход.
    let mut longer = tree.clone();
    let leaf = longer.main_line_end();
    if longer.get(leaf).unwrap().outcome().is_none() {
        let m = legal_moves(longer.get(leaf).unwrap().position())[0];
        longer.add_move(leaf, &m).unwrap();
        assert!(!tree.same_structure(&longer));
    }
    // Другая начальная позиция.
    let other = GameTree::from_position(pos("W:Wa1:Bh8"));
    assert!(!GameTree::new().same_structure(&other));
    // Положение указателя не влияет на сравнение.
    let mut moved = tree.clone();
    moved.go_to(moved.len() - 1);
    assert!(tree.same_structure(&moved));
}

// ---------------------------------------------------------------------------
// PDN: запись
// ---------------------------------------------------------------------------

#[test]
fn pdn_text_has_russian_game_type_and_variations_in_parentheses() {
    let mut tree = GameTree::new();
    let ids = sample_line(&mut tree);
    tree.go_to(GameTree::ROOT);
    play_all(&mut tree, &["e3-f4", "b6-a5"]);
    tree.go_to(ids[2]);
    play(&mut tree, "e7:g5");

    let tags = standard_tags(Color::White, "Игрок", "Компьютер, уровень 4");
    let text = to_pdn(&tree, &tags);
    assert!(text.contains("[GameType \"25,W,8,8,A0,0\"]"), "{text}");
    assert_eq!(GAME_TYPE, "25,W,8,8,A0,0");
    assert!(text.contains("[White \"Игрок\"]"));
    assert!(text.contains("[Black \"Компьютер, уровень 4\"]"));
    assert!(text.contains("[WhiteType \"human\"]") && text.contains("[BlackType \"program\"]"));
    assert!(text.contains("[Result \"*\"]"));
    assert!(
        !text.contains("[FEN"),
        "стандартная начальная позиция не записывается"
    );
    let movetext = text.split("\n\n").nth(1).unwrap().trim();
    assert_eq!(
        movetext,
        "1. c3-d4 ( 1. e3-f4 b6-a5 ) 1... f6-e5 2. d4:f6 g7:e5 ( 2... e7:g5 ) *"
    );

    let game = from_pdn(&text).unwrap();
    assert!(game.tree.same_structure(&tree));
    assert_eq!(game.human_color(), Some(Color::White));
    assert_eq!(game.tag("White"), Some("Игрок"));
    let black = to_pdn(&tree, &standard_tags(Color::Black, "Игрок", "Компьютер"));
    assert_eq!(from_pdn(&black).unwrap().human_color(), Some(Color::Black));
    assert_eq!(from_pdn("1. c3-d4 *").unwrap().human_color(), None);
}

#[test]
fn pdn_result_matches_the_end_of_the_main_line() {
    let mut tree = GameTree::from_position(pos("W:Wc3:Bd4"));
    play(&mut tree, "c3:e5");
    let text = to_pdn(&tree, &[]);
    assert!(text.contains("[Result \"1-0\"]"), "{text}");
    assert!(text.trim_end().ends_with("1. c3:e5 1-0"), "{text}");
    assert!(
        text.contains("[FEN \"W:Wc3:Bd4\"]"),
        "нестандартная начальная позиция записана"
    );
    assert!(from_pdn(&text).unwrap().tree.same_structure(&tree));

    let mut tree = GameTree::from_position(pos("B:Wd4:Be5"));
    play(&mut tree, "e5:c3");
    let text = to_pdn(&tree, &[]);
    assert!(text.contains("[Result \"0-1\"]"));
    assert!(
        text.contains("1... e5:c3 0-1"),
        "первый ход чёрных записан с многоточием: {text}"
    );
    assert!(from_pdn(&text).unwrap().tree.same_structure(&tree));

    assert_eq!(
        result_token(Some(Outcome::Draw(DrawReason::Repetition))),
        "1/2-1/2"
    );
    assert_eq!(result_token(None), "*");
}

#[test]
fn long_games_are_wrapped_into_lines() {
    let mut rng = Rng::new(11);
    let tree = random_tree(&mut rng, Position::initial(), 400);
    let text = to_pdn(&tree, &[]);
    assert!(
        text.lines().all(|line| line.chars().count() <= 80),
        "строки не длиннее 80 знаков"
    );
    assert!(text.lines().count() > 8);
}

// ---------------------------------------------------------------------------
// PDN: сохранение и загрузка дают идентичное дерево
// ---------------------------------------------------------------------------

#[test]
fn pdn_round_trip_preserves_random_trees_with_variations() {
    let mut rng = Rng::new(0x9D11);
    let starts = [
        Position::initial(),
        pos("B:Wa1,c1,e1,g1,b2,d2,f2,h2,a3,c3,e3,g3:Bb6,d6,f6,h6,a7,c7,e7,g7,b8,d8,f8,h8"),
        pos("W:WKa1,Kc1,e3,g3:BKh8,Kf8,b6,d6"),
        pos("B:WKd2,c3,e3:BKe7,f6,d6"),
    ];
    let mut with_variations = 0;
    for round in 0..200 {
        let start = starts[round % starts.len()];
        let steps = 5 + rng.below(250);
        let tree = random_tree(&mut rng, start, steps);
        if max_children(&tree) > 1 {
            with_variations += 1;
        }
        let text = to_pdn(&tree, &standard_tags(Color::White, "A", "B"));
        let game = from_pdn(&text).unwrap_or_else(|e| panic!("раунд {round}: {e}\n{text}"));
        assert!(
            game.tree.same_structure(&tree),
            "раунд {round}: дерево изменилось\n{text}"
        );
        assert_eq!(game.tree.len(), tree.len());
        // Повторное сохранение даёт тот же текст.
        assert_eq!(to_pdn(&game.tree, &game.tags), text, "раунд {round}");
        // Загрузка из байтов с BOM и переводами строк Windows.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
        assert!(from_bytes(&bytes).unwrap().tree.same_structure(&tree));
    }
    assert!(
        with_variations > 150,
        "в большинстве деревьев есть варианты: {with_variations}"
    );
}

#[test]
fn pdn_round_trip_through_a_file() {
    let mut rng = Rng::new(77);
    let tree = random_tree(&mut rng, Position::initial(), 150);
    let path = std::env::temp_dir().join(format!("rucheckers-test-{}.pdn", std::process::id()));
    std::fs::write(
        &path,
        to_pdn(&tree, &standard_tags(Color::Black, "Игрок", "Компьютер")),
    )
    .unwrap();
    let game = from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(game.tree.same_structure(&tree));
    assert_eq!(game.human_color(), Some(Color::Black));
}

// ---------------------------------------------------------------------------
// PDN: чтение записей других программ
// ---------------------------------------------------------------------------

#[test]
fn pdn_reader_accepts_common_dialects() {
    let text = "\u{feff}[Event \"Турнир\"]\r\n[GameType \"25\"]\r\n% служебная строка\r\n\r\n\
        1.c3-d4 {комментарий (со скобками)} f6-e5! 2. d4xf6 $1 g7xe5 ; до конца строки ( x )\r\n\
        3. b2-c3 ?! 3... h8-g7 1/2-1/2\r\n\r\n[Event \"вторая партия\"]\r\n1. e3-f4 *";
    let game = from_pdn(text.trim_start_matches('\u{feff}')).unwrap();
    let line: Vec<String> = game
        .tree
        .path_to(game.tree.main_line_end())
        .iter()
        .filter_map(|&id| game.tree.get(id).unwrap().mv().map(format_move))
        .collect();
    assert_eq!(line, ["c3-d4", "f6-e5", "d4:f6", "g7:e5", "b2-c3", "h8-g7"]);
    assert_eq!(game.tree.len(), 7, "вторая партия файла не читается");
    assert_eq!(game.tag("event"), Some("Турнир"));

    // Сокращённая запись взятия (только начало и конец) и многоточие отдельным словом.
    let error = from_pdn("[FEN \"B:Wd4:Be5\"]\n1. ... e5:g7 *").unwrap_err();
    assert!(
        matches!(
            error,
            PdnError::BadMove {
                problem: MoveProblem::Illegal,
                ..
            }
        ),
        "{error:?}"
    );
    let game = from_pdn("[FEN \"W:Wc3:Bd4,f6\"]\n1. c3:g7 *").unwrap();
    assert_eq!(game.tree.len(), 2);
    let game = from_pdn("[FEN \"B:Wd4:Be5\"]\n1. ... e5xc3 0-1").unwrap();
    assert_eq!(game.tree.len(), 2);
    // Пустая партия с одним заголовком допустима.
    let game = from_pdn("[White \"A\"]\n[Black \"B\"]\n*").unwrap();
    assert!(game.tree.is_empty());
}

// ---------------------------------------------------------------------------
// PDN: повреждённые и посторонние файлы
// ---------------------------------------------------------------------------

#[test]
fn damaged_and_foreign_files_give_errors_not_panics() {
    let line1 = |problem| PdnError::BadMove {
        line: 1,
        token: String::new(),
        problem,
    };
    let cases: Vec<(&str, PdnError)> = vec![
        ("", PdnError::Empty),
        ("  \n\t\r\n ", PdnError::Empty),
        ("Привет, это не партия", PdnError::NotPdn),
        ("<html><body>1-0</body></html>", PdnError::NotPdn),
        ("{\"json\": true}", PdnError::NotPdn),
        ("[Event \"x\"", PdnError::BadTag { line: 1 }),
        ("[Event x]", PdnError::BadTag { line: 1 }),
        (
            "[Event \"x\"]\n[Site \"незакрытая строка]\n1. c3-d4",
            PdnError::BadTag { line: 2 },
        ),
        ("[]", PdnError::BadTag { line: 1 }),
        (
            "[GameType \"20,W,10,10,N2,0\"]\n1. 32-28 19-23 *",
            PdnError::WrongGameType("20,W,10,10,N2,0".into()),
        ),
        ("[GameType \"шашки\"]\n*", PdnError::WrongGameType("шашки".into())),
        ("[FEN \"W:Wz9:Bh8\"]\n*", PdnError::BadFen("W:Wz9:Bh8".into())),
        ("[FEN \"\"]\n*", PdnError::BadFen(String::new())),
        ("1. c3-d4 f6-e5 2. a1-a2 *", line1(MoveProblem::Syntax)),
        ("1. c3-d4 f6-e5 2. a1-b2 *", line1(MoveProblem::Illegal)),
        ("1. c3-d4 c3-d4 *", line1(MoveProblem::Illegal)),
        ("1. c3-d4 ход *", line1(MoveProblem::Syntax)),
        ("1. c3-d4 ( 1. e3-f4", PdnError::UnbalancedVariation { line: 1 }),
        ("1. c3-d4 ) f6-e5", PdnError::UnbalancedVariation { line: 1 }),
        ("( 1. c3-d4 )", PdnError::VariationWithoutMove { line: 1 }),
        (
            "1. c3-d4 ( ( 1. e3-f4 ) )",
            PdnError::VariationWithoutMove { line: 1 },
        ),
        (
            "1. c3-d4 { незакрытый комментарий",
            PdnError::UnterminatedComment { line: 1 },
        ),
        (
            "[FEN \"W:Wc3:Bd4\"]\n\n1. c3:e5 a1-b2 *",
            PdnError::BadMove {
                line: 3,
                token: String::new(),
                problem: MoveProblem::Illegal,
            },
        ),
    ];
    for (text, expected) in cases {
        let error = from_pdn(text).expect_err(text);
        match (&error, &expected) {
            (
                PdnError::BadMove {
                    line: l1,
                    problem: p1,
                    ..
                },
                PdnError::BadMove {
                    line: l2,
                    problem: p2,
                    ..
                },
            ) => assert_eq!((l1, p1), (l2, p2), "{text:?}"),
            _ => assert_eq!(error, expected, "{text:?}"),
        }
        // Сообщение об ошибке формируется без сбоя.
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn moves_after_an_automatic_draw_are_reported() {
    let cycle = "g1-h2 b8-a7 h2-g1 a7-b8 ";
    let text = format!("[FEN \"W:WKg1,a1,c1,e1:BKb8,d8,f8,h8\"]\n{cycle}{cycle}g1-h2 *");
    let error = from_pdn(&text).unwrap_err();
    assert!(
        matches!(
            error,
            PdnError::BadMove {
                problem: MoveProblem::GameOver,
                ..
            }
        ),
        "{error:?}"
    );
    // Та же запись без лишнего хода загружается.
    let text = format!("[FEN \"W:WKg1,a1,c1,e1:BKb8,d8,f8,h8\"]\n{cycle}{cycle}1/2-1/2");
    let game = from_pdn(&text).unwrap();
    assert_eq!(
        game.tree.get(game.tree.main_line_end()).unwrap().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
}

#[test]
fn binary_and_oversized_files_are_rejected() {
    // Заголовок PNG и случайные байты.
    let png = [
        0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
    ];
    assert_eq!(from_bytes(&png).unwrap_err(), PdnError::NotPdn);
    let mut rng = Rng::new(3);
    for _ in 0..200 {
        let len = rng.below(600);
        let bytes: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let _ = from_bytes(&bytes);
    }
    assert_eq!(from_bytes(&[0u8; 64]).unwrap_err(), PdnError::NotPdn);
    assert_eq!(
        from_bytes(&[0xFF, 0xFE, b'1', 0, b'.', 0]).unwrap_err(),
        PdnError::NotPdn,
        "UTF-16"
    );

    let huge = vec![b' '; MAX_FILE_SIZE + 1];
    assert_eq!(from_bytes(&huge).unwrap_err(), PdnError::TooLarge);
    // Глубокая вложенность скобок и очень длинные слова не приводят к сбою.
    let nested = format!("1. c3-d4 {}", "(".repeat(50_000));
    assert!(from_pdn(&nested).is_err());
    let long_word = "c".repeat(200_000);
    assert!(from_pdn(&format!("1. {long_word} *")).is_err());
    let many_tags = "[A \"b\"]\n".repeat(20_000);
    assert!(from_pdn(&many_tags).unwrap().tree.is_empty());
}

#[test]
fn randomly_corrupted_pdn_never_panics() {
    let mut rng = Rng::new(0xBAD_F11E);
    let mut sources = Vec::new();
    for _ in 0..12 {
        let steps = 20 + rng.below(200);
        let tree = random_tree(&mut rng, Position::initial(), steps);
        sources.push(to_pdn(&tree, &standard_tags(Color::White, "Игрок", "Компьютер")).into_bytes());
    }
    let alphabet = b"()[]{}\";.:-x*$!?%\n\r\t 0123456789abcdefghKWB\\\xFF\x00";
    let (mut accepted, mut rejected) = (0u32, 0u32);
    for i in 0..6_000 {
        let mut bytes = sources[i % sources.len()].clone();
        for _ in 0..=rng.below(4) {
            if bytes.is_empty() {
                break;
            }
            let at = rng.below(bytes.len());
            match rng.below(5) {
                0 => bytes[at] = alphabet[rng.below(alphabet.len())],
                1 => bytes.insert(at, alphabet[rng.below(alphabet.len())]),
                2 => {
                    bytes.remove(at);
                }
                3 => bytes.truncate(at),
                _ => {
                    let end = (at + rng.below(40)).min(bytes.len());
                    let chunk = bytes[at..end].to_vec();
                    let to = rng.below(bytes.len());
                    bytes.splice(to..to, chunk);
                }
            }
        }
        match from_bytes(&bytes) {
            Ok(game) => {
                accepted += 1;
                // Принятое дерево самосогласованно: его можно сохранить и прочитать снова.
                let again = from_pdn(&to_pdn(&game.tree, &game.tags)).unwrap();
                assert!(again.tree.same_structure(&game.tree));
            }
            Err(error) => {
                rejected += 1;
                assert!(!error.to_string().is_empty());
            }
        }
    }
    assert!(
        accepted > 0 && rejected > 1_000,
        "принято {accepted}, отклонено {rejected}"
    );
}
