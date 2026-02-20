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

CREATE TABLE IF NOT EXISTS `hits` (
  `pageid` varchar(100) NOT NULL,
  `isunique` tinyint(1) NOT NULL,
  `hitcount` int(10) unsigned NOT NULL,
  KEY `pageid` (`pageid`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

CREATE TABLE IF NOT EXISTS `nodupes` (
  `ids_hash` char(64) NOT NULL,
  `time` bigint(20) unsigned NOT NULL,
  PRIMARY KEY (`ids_hash`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

-- ============================================================================
-- upvotes - Page upvote/downvote system
-- ============================================================================

CREATE DATABASE IF NOT EXISTS `upvotes` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

CREATE USER IF NOT EXISTS 'upvotes'@'%' IDENTIFIED BY 'dev_upvotes_pass';
GRANT ALL PRIVILEGES ON `upvotes`.* TO 'upvotes'@'%';

USE `upvotes`;

CREATE TABLE IF NOT EXISTS `counts` (
  `category` varchar(255) NOT NULL,
  `permanent_id` varchar(255) NOT NULL,
  `title` text NOT NULL,
  `description` text NOT NULL,
  `canonical_url` text NOT NULL,
  `upvotes` int(11) NOT NULL,
  `downvotes` int(11) NOT NULL,
  KEY `category` (`category`,`permanent_id`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 COLLATE=latin1_swedish_ci;

CREATE TABLE IF NOT EXISTS `history` (
  `hash` varchar(255) NOT NULL,
  `action` varchar(10) NOT NULL,
  `time_added` bigint(20) NOT NULL,
  UNIQUE KEY `hash` (`hash`)
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
