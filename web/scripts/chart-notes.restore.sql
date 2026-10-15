UPDATE `chart` c
JOIN `chart_notes_backup` b ON b.`sha256` = c.`sha256`
SET c.`notes` = b.`old_notes`
WHERE c.`notes` = b.`new_notes`;
