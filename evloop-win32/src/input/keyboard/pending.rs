use std::sync::{
	Mutex,
	atomic::{AtomicU32, Ordering::Relaxed},
};

enum PendingMessage<T> {
	Incomplete,
	Complete(T),
}

struct IdentifiedPendingMessage<T> {
	token: PendingMessageToken,
	msg: PendingMessage<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingMessageToken(u32);

/// While processing keyboard events, we sometimes need
/// to call `PeekMessageW` (`next_msg`). But `PeekMessageW`
/// can also call the event handler, which means that the new event
/// gets processed before finishing to process the one that came before.
///
/// This would mean that the application receives events in the wrong order.
/// To avoid this, we keep track whether we are in the middle of processing
/// an event. Such an event is an "incomplete pending event". A
/// "complete pending event" is one that has already finished processing, but
/// hasn't been dispatched to the application because there still are incomplete
/// pending events that came before it.
///
/// When we finish processing an event, we call `complete_pending`,
/// which returns an empty array if there are incomplete pending events, but
/// if all pending events are complete, then it returns all pending events in
/// the order they were encountered. These can then be dispatched to the application
pub struct PendingEventQueue<T> {
	pending: Mutex<Vec<IdentifiedPendingMessage<T>>>,
	next_id: AtomicU32,
}

impl<T> PendingEventQueue<T> {
	/// Add a new pending event to the "pending queue"
	pub fn add_pending(&self) -> PendingMessageToken {
		let token = self.next_token();
		let mut pending = self.pending.lock().unwrap();
		pending.push(IdentifiedPendingMessage { token, msg: PendingMessage::Incomplete });
		token
	}

	/// Returns all finished pending events
	///
	/// If the return value is non empty, it's guaranteed to contain `msg`
	///
	/// See also: `add_pending`
	pub fn complete_pending(&self, token: PendingMessageToken, msg: T) -> Vec<T> {
		let mut pending = self.pending.lock().unwrap();
		let mut target_is_first = false;
		for (i, pending_msg) in pending.iter_mut().enumerate() {
			if pending_msg.token == token {
				pending_msg.msg = PendingMessage::Complete(msg);
				if i == 0 {
					target_is_first = true;
				}
				break;
			}
		}
		if target_is_first {
			// If the message that we just finished was the first one in the pending queue,
			// then we can empty the queue, and dispatch all of the messages.
			Self::drain_pending(&mut *pending)
		} else {
			Vec::new()
		}
	}

	pub fn complete_multi(&self, msgs: Vec<T>) -> Vec<T> {
		let mut pending = self.pending.lock().unwrap();
		if pending.is_empty() {
			return msgs;
		}
		pending.reserve(msgs.len());
		for msg in msgs {
			pending.push(IdentifiedPendingMessage {
				token: self.next_token(),
				msg: PendingMessage::Complete(msg),
			});
		}
		Vec::new()
	}

	/// Returns all finished pending events
	///
	/// It's safe to call this even if the element isn't in the list anymore
	///
	/// See also: `add_pending`
	pub fn remove_pending(&self, token: PendingMessageToken) -> Vec<T> {
		let mut pending = self.pending.lock().unwrap();
		let mut was_first = false;
		if let Some(m) = pending.first() {
			if m.token == token {
				was_first = true;
			}
		}
		pending.retain(|m| m.token != token);
		if was_first { Self::drain_pending(&mut *pending) } else { Vec::new() }
	}

	fn drain_pending(pending: &mut Vec<IdentifiedPendingMessage<T>>) -> Vec<T> {
		pending
			.drain(..)
			.map(|m| match m.msg {
				PendingMessage::Complete(msg) => msg,
				PendingMessage::Incomplete => {
					panic!(
						"Found an incomplete pending message when collecting messages. This \
                         indicates a bug in winit."
					)
				}
			})
			.collect()
	}

	fn next_token(&self) -> PendingMessageToken {
		// It's okay for the u32 to overflow here. Yes, that could mean
		// that two different messages have the same token,
		// but that would only happen after having about 4 billion
		// messages sitting in the pending queue.
		//
		// In that case, having two identical tokens is the least of your concerns.
		let id = self.next_id.fetch_add(1, Relaxed);
		PendingMessageToken(id)
	}
}

impl<T> Default for PendingEventQueue<T> {
	fn default() -> Self {
		PendingEventQueue { pending: Mutex::new(Vec::new()), next_id: AtomicU32::new(0) }
	}
}
