-- Schema initialization for defuse-rust development environment.
-- This creates all databases, tables, and users needed to run the application.
--
-- Passwords here are dev-only placeholders. Production uses different credentials.

-- ============================================================================
-- phpcount - Page hit counter
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `phpcount` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

CREATE USER IF NOT EXISTS 'phpcount'@'%' IDENTIFIED BY 'dev_phpcount_pass';
GRANT ALL PRIVILEGES ON `phpcount`.* TO 'phpcount'@'%';

USE `phpcount`;

-- PRIMARY KEY (`pageid`, `isunique`) is upstream PHPCount's, and it is load-bearing:
-- `create_counts_if_not_present` is a check-then-insert, so without a unique constraint
-- two concurrent first-hits on a page each insert a counter row. The duplicate is
-- permanent, `count_hit` increments every matching row forever, and `get_hit_counts`
-- reads whichever one LIMIT 1 returns. With the key the losing INSERT fails instead,
-- which is logged and dropped -- the behaviour the code was written against.
CREATE TABLE IF NOT EXISTS `hits` (
  `pageid` varchar(100) NOT NULL,
  `isunique` tinyint(1) NOT NULL,
  `hitcount` int(10) unsigned NOT NULL,
  PRIMARY KEY (`pageid`, `isunique`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- KEY (`time`) is what keeps the expiry sweep off a full table scan. `cleanup` runs
-- `DELETE FROM nodupes WHERE time < ?` on every counted page view, under a MyISAM
-- table-level write lock, so without it the whole site serialises behind that scan.
-- Measured on crackstation's equivalent table, 23 MB over ~200k live rows: 20 ms
-- without the index, 1 ms with it.
CREATE TABLE IF NOT EXISTS `nodupes` (
  `ids_hash` char(64) NOT NULL,
  `time` bigint(20) unsigned NOT NULL,
  PRIMARY KEY (`ids_hash`),
  KEY `time` (`time`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- ============================================================================
-- upvotes - Page upvote/downvote system
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `upvotes` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

CREATE USER IF NOT EXISTS 'upvotes'@'%' IDENTIFIED BY 'dev_upvotes_pass';
GRANT ALL PRIVILEGES ON `upvotes`.* TO 'upvotes'@'%';

USE `upvotes`;

-- UNIQUE KEY (`permanent_id`) is the identity every vote query uses: all six vote
-- UPDATEs and the DELETE filter on `permanent_id` alone, so `KEY (category,
-- permanent_id)` cannot serve them -- permanent_id is not its leftmost column. It also
-- enforces what `ensure_page`'s INSERT ... WHERE NOT EXISTS was upholding by
-- convention: two rows for one page would have every UPDATE increment both while the
-- page displays one, the same failure the `hits` primary key prevents.
CREATE TABLE IF NOT EXISTS `counts` (
  `category` varchar(255) NOT NULL,
  `permanent_id` varchar(255) NOT NULL,
  `title` text NOT NULL,
  `description` text NOT NULL,
  `canonical_url` text NOT NULL,
  `upvotes` int(11) NOT NULL,
  `downvotes` int(11) NOT NULL,
  UNIQUE KEY `permanent_id` (`permanent_id`),
  KEY `category` (`category`,`permanent_id`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- KEY (`time_added`) keeps `remove_old_vote_history` off a full table scan. It runs
-- from `get_vote_state`, so it is on every page render that shows vote buttons, not
-- just on votes -- the same shape as the `nodupes` sweep and on a hotter path.
CREATE TABLE IF NOT EXISTS `history` (
  `hash` varchar(255) NOT NULL,
  `action` varchar(10) NOT NULL,
  `time_added` bigint(20) NOT NULL,
  UNIQUE KEY `hash` (`hash`),
  KEY `time_added` (`time_added`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- ============================================================================
-- cracky_bin - Encrypted pastebin
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `cracky_bin` DEFAULT CHARACTER SET latin1 COLLATE latin1_swedish_ci;

CREATE USER IF NOT EXISTS 'cracky_bin'@'%' IDENTIFIED BY 'dev_pastebin_pass';
GRANT ALL PRIVILEGES ON `cracky_bin`.* TO 'cracky_bin'@'%';

USE `cracky_bin`;

CREATE TABLE IF NOT EXISTS `pastes` (
  `token` char(64) NOT NULL,
  `data` longtext NOT NULL,
  `time` int(11) NOT NULL,
  `jscrypt` tinyint(1) NOT NULL,
  PRIMARY KEY (`token`),
  KEY `time` (`time`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- ============================================================================
-- cracky_trent - TRENT random number drawing system
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `cracky_trent` DEFAULT CHARACTER SET latin1 COLLATE latin1_swedish_ci;

CREATE USER IF NOT EXISTS 'cracky_trent'@'%' IDENTIFIED BY 'dev_trent_pass';
GRANT ALL PRIVILEGES ON `cracky_trent`.* TO 'cracky_trent'@'%';

USE `cracky_trent`;

CREATE TABLE IF NOT EXISTS `drawings` (
  `drawingnum` int(11) NOT NULL AUTO_INCREMENT,
  `complete` tinyint(1) NOT NULL,
  `passwordhash` char(64) NOT NULL,
  `starttime` int(11) unsigned NOT NULL,
  `reviewtime` int(10) unsigned NOT NULL,
  `printout` longtext NOT NULL,
  `userprintout` longtext NOT NULL,
  PRIMARY KEY (`drawingnum`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- ============================================================================
-- timecapsule - Encrypted time capsule messages
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `timecapsule` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

CREATE USER IF NOT EXISTS 'timecapsule'@'%' IDENTIFIED BY 'dev_timecapsule_pass';
GRANT ALL PRIVILEGES ON `timecapsule`.* TO 'timecapsule'@'%';

USE `timecapsule`;

CREATE TABLE IF NOT EXISTS `timecapsule` (
  `id` bigint(20) NOT NULL AUTO_INCREMENT,
  `timestamp` bigint(20) NOT NULL,
  `message` longtext NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3 COLLATE=utf8mb3_bin;

-- ============================================================================

FLUSH PRIVILEGES;
