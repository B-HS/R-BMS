ALTER TABLE `chart_best` ADD `scratch_auto` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `course_best` ADD `scratch_auto` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `course_score` ADD `scratch_auto` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `chart_best` DROP PRIMARY KEY, ADD PRIMARY KEY(`chart_sha256`,`user_id`,`scratch_auto`);--> statement-breakpoint
ALTER TABLE `course_best` DROP PRIMARY KEY, ADD PRIMARY KEY(`course_hash`,`user_id`,`scratch_auto`);--> statement-breakpoint
DROP INDEX `chart_best_rank_idx` ON `chart_best`;--> statement-breakpoint
CREATE INDEX `chart_best_rank_idx` ON `chart_best` (`chart_sha256`,`scratch_auto`,`clear`,`ex_score`);