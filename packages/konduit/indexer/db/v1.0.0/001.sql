CREATE TABLE block (
  block_no    INTEGER UNIQUE NOT NULL,
  header_hash BLOB UNIQUE NOT NULL,
  slot_no     INTEGER NOT NULL,

  CHECK (length(header_hash) = 32),
  PRIMARY KEY (slot_no)
);

-- Channel thread is uniquely identified by the initial output on the chain.
-- It **should** be unique by the `keytag (`add_key + tag`) as well but this
-- can not be enforced on the chain (like "costly" thread token) and we can expect
-- for example malicious "mimics".
CREATE TABLE channel (
    add_vkey          BLOB NOT NULL,
    block_slot_no     INTEGER NOT NULL REFERENCES block(slot_no) ON DELETE CASCADE,
    datum             BLOB NOT NULL,
    lovelace          INTEGER NOT NULL,
    output_index      INTEGER NOT NULL,
    script_hash       BLOB NOT NULL,
    sub_vkey          BLOB NOT NULL,
    tag               BLOB NOT NULL,
    transaction_id    BLOB NOT NULL,
    transaction_index INTEGER NOT NULL, -- index of the transaction in the block

    UNIQUE (block_slot_no, transaction_index),
    CHECK (
      length(script_hash) = 28
      AND length(transaction_id) = 32
      AND length(add_vkey) = 32
      AND length(sub_vkey) = 32
      AND lovelace > 0
      AND output_index >= 0
    ),
    PRIMARY KEY (transaction_id, output_index)
);

CREATE TRIGGER channel_no_updates
BEFORE UPDATE ON channel
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'channel table does not allow updates');
END;

CREATE INDEX channel_by_add_vkey ON channel(add_vkey);
CREATE INDEX channel_by_sub_vkey ON channel(sub_vkey);
CREATE INDEX channel_by_keytag ON channel(add_vkey, tag);

CREATE TABLE focus (
    add_vkey         BLOB NOT NULL,
    tag              BLOB NOT NULL,
    transaction_id     BLOB NOT NULL,
    transaction_index  INTEGER NOT NULL,

    FOREIGN KEY (transaction_id, transaction_index)
      REFERENCES channel (transaction_id, transaction_index)
      ON DELETE CASCADE,

    UNIQUE (add_vkey, tag),
    PRIMARY KEY (transaction_id, transaction_index)
);

CREATE TRIGGER focus_no_updates
BEFORE UPDATE ON focus
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'focus table does not allow updates');
END;

CREATE TRIGGER focus_sync_add_vkey_and_tag
BEFORE INSERT ON focus
FOR EACH ROW
BEGIN
    SET NEW.add_vkey = (
      SELECT add_vkey FROM channel
      WHERE transaction_id = NEW.transaction_id
        AND transaction_index = NEW.transaction_index
    );
    SET NEW.tag = (
      SELECT tag FROM channel
      WHERE transaction_id = NEW.transaction_id
        AND transaction_index = NEW.transaction_index
    );
END;

CREATE TABLE step (
    block_slot_no           INTEGER NOT NULL REFERENCES block(slot_no) ON DELETE CASCADE,
    channel_transaction_id  BLOB NOT NULL,
    channel_output_index    INTEGER NOT NULL,
    datum                   BLOB, -- NULL only for close
    lovelace                INTEGER, -- NULL only for close
    output_index            INTEGER, -- NULL only for close
    redeemer                BLOB,
    transaction_id          BLOB NOT NULL,
    transaction_index       INTEGER NOT NULL, -- index of the transaction in the block

    FOREIGN KEY (channel_transaction_id, channel_output_index)
      REFERENCES channel (transaction_id, output_index)
      ON DELETE CASCADE,

    UNIQUE (block_slot_no, transaction_index), -- Used for sorting the steps across and in one block
    CHECK (
      (datum IS NOT NULL AND output_index IS NOT NULL AND redeemer IS NOT NULL AND lovelace IS NOT NULL) OR
      (datum IS NULL AND output_index IS NULL AND redeemer IS NOT NULL AND lovelace IS NULL)
    ),

    CHECK (
      length(transaction_id) = 32
      AND lovelace IS NULL OR lovelace > 0
      AND output_index IS NULL OR output_index >= 0
      AND transaction_index >= 0
    ),
    PRIMARY KEY (transaction_id, output_index)
);

CREATE TRIGGER step_no_updates
BEFORE UPDATE ON step
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'step table does not allow updates');
END;

CREATE TRIGGER no_step_after_close
BEFORE INSERT ON step
FOR EACH ROW
WHEN
  ( -- Trigger if `close` is already present and
    -- the new record is inserted after it.
    EXISTS (
      SELECT 1 FROM step
      WHERE
        channel_transaction_id = NEW.channel_transaction_id
        AND channel_output_index = NEW.channel_output_index
        AND output_index IS NULL
        AND (
          block_slot_no < NEW.block_slot_no
          OR (
            block_slot_no = NEW.block_slot_no
            AND transaction_index < NEW.transaction_index
          )
        )
    )
  ) OR (
    -- Trigger if we are inserting `close` and some
    -- existing `step` exists after it.
    NEW.output_index IS NULL
    AND EXISTS (
      SELECT 1 FROM step
      WHERE
        channel_transaction_id = NEW.channel_transaction_id
        AND channel_output_index = NEW.channel_output_index
        AND (
          block_slot_no > NEW.block_slot_no
          OR (
            block_slot_no = NEW.block_slot_no
            AND transaction_index > NEW.transaction_index
          )
        )
    )
  )
BEGIN
  SELECT RAISE(ABORT, 'Any step after a `close` is not possible');
END;

CREATE TRIGGER no_step_before_channel
BEFORE INSERT ON step
FOR EACH ROW
WHEN
  -- Trigger if we are inserting a `step`
  -- before the initial `channel` output.
  EXISTS (
    SELECT 1 FROM channel
    WHERE
      transaction_id = NEW.channel_transaction_id
      AND output_index = NEW.channel_output_index
      AND (
        block_slot_no > NEW.block_slot_no
        OR (
          block_slot_no = NEW.block_slot_no
          AND transaction_index > NEW.transaction_index
        )
      )
  )
BEGIN
  SELECT RAISE(ABORT, 'Any step before an initial `channel` output is not possible');
END;

