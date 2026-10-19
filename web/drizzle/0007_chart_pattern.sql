CREATE TABLE `chart_pattern` (
	`md5` char(32) NOT NULL,
	`sha256` char(64) NOT NULL,
	`mode` varchar(16) NOT NULL,
	`format_version` int NOT NULL,
	`parser_version` int NOT NULL,
	`playable_notes` int NOT NULL DEFAULT 0,
	`note_count` int NOT NULL DEFAULT 0,
	`content_hash` char(64) NOT NULL,
	`byte_size` int NOT NULL DEFAULT 0,
	`data` mediumblob NOT NULL,
	`features` json,
	`features_version` int NOT NULL DEFAULT 0,
	`uploaded_by` varchar(36),
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	`updated_at` bigint NOT NULL DEFAULT 0,
	CONSTRAINT `chart_pattern_md5` PRIMARY KEY(`md5`)
);
--> statement-breakpoint
CREATE INDEX `chart_pattern_sha256_idx` ON `chart_pattern` (`sha256`);