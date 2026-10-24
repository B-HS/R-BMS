ALTER TABLE `user_dan` ADD `assisted` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `user_dan` DROP PRIMARY KEY, ADD PRIMARY KEY(`user_id`,`system`,`label`,`assisted`);--> statement-breakpoint
ALTER TABLE `user_dan_display` ADD `assisted` boolean DEFAULT false NOT NULL;