CREATE TABLE `chart_blocklist` (
	`sha256` char(64) NOT NULL,
	`md5` char(32) NOT NULL,
	`source_id` varchar(64) NOT NULL,
	`source_label` varchar(200) NOT NULL,
	`path` varchar(512) NOT NULL,
	`added_by` varchar(36),
	`created_at` timestamp(3) NOT NULL DEFAULT (now()),
	CONSTRAINT `chart_blocklist_sha256` PRIMARY KEY(`sha256`),
	CONSTRAINT `chart_blocklist_md5_uniq` UNIQUE(`md5`)
);
--> statement-breakpoint
CREATE INDEX `chart_blocklist_source_idx` ON `chart_blocklist` (`source_id`);