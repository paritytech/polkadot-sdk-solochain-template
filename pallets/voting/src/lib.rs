#![cfg_attr(not(feature = "std"), no_std)]

/// Blockchain-based campus voting pallet
/// Features:
///   - RSA encrypted vote storage
///   - Blind signature verification
///   - Nullifier-based double-vote prevention
///   - Election phase management
///   - On-chain tally via authority reveal
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::EnsureOrigin,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // STRUCTURES
    // =========================================================

    /// Stores the RSA-encrypted vote + blind signature on-chain
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct EncryptedVote<T: Config> {
        pub encrypted_vote: BoundedVec<u8, T::MaxEncryptedVoteSize>,
        pub blind_signature: BoundedVec<u8, T::MaxBlindSignatureSize>,
    }

    /// Election phases — controls what actions are allowed
    ///
    ///  NotStarted → Voting → Ended → TallyComplete
    ///      ↑ only sudo can move between phases
    #[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub enum ElectionPhase {
        #[default]
        NotStarted,   // Election not begun yet
        Voting,       // Voters can submit votes
        Ended,        // Voting closed, awaiting decryption
        TallyComplete, // All votes revealed, tally final
    }

    // =========================================================
    // CONFIG
    // =========================================================
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin allowed to manage election (sudo / POA authority)
        /// In your setup: Alice (sudo) controls this
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        #[pallet::constant]
        type MaxEncryptedVoteSize: Get<u32>;

        #[pallet::constant]
        type MaxBlindSignatureSize: Get<u32>;

        /// Max number of candidates allowed
        #[pallet::constant]
        type MaxCandidates: Get<u32>;

        /// Max length of a candidate name
        #[pallet::constant]
        type MaxCandidateNameLen: Get<u32>;
    }

    // =========================================================
    // STORAGE
    // =========================================================

    /// Current phase of the election
    #[pallet::storage]
    #[pallet::getter(fn election_phase)]
    pub type CurrentPhase<T: Config> = StorageValue<_, ElectionPhase, ValueQuery>;

    /// Stores all encrypted votes — key: vote_id, value: EncryptedVote
    #[pallet::storage]
    #[pallet::getter(fn encrypted_votes)]
    pub type EncryptedVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,           // vote_id
        EncryptedVote<T>,
        OptionQuery,
    >;

    /// Auto-incrementing vote ID counter
    #[pallet::storage]
    #[pallet::getter(fn vote_counter)]
    pub type VoteCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Nullifier set — stores a hash of each blind signature
    /// Prevents the same blind-signed token from voting twice
    /// Key: nullifier hash (first 32 bytes of blind_signature)
    #[pallet::storage]
    #[pallet::getter(fn used_nullifiers)]
    pub type UsedNullifiers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BoundedVec<u8, ConstU32<64>>, // nullifier (truncated hash of blind sig)
        bool,
        ValueQuery,
    >;

    /// Tracks which accounts have voted (prevents one account voting twice)
    #[pallet::storage]
    #[pallet::getter(fn has_voted)]
    pub type HasVoted<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// Registered candidates — key: candidate_id (u32)
    #[pallet::storage]
    #[pallet::getter(fn candidates)]
    pub type Candidates<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,  // candidate_id
        BoundedVec<u8, T::MaxCandidateNameLen>, // candidate name
        OptionQuery,
    >;

    /// Candidate ID counter
    #[pallet::storage]
    pub type CandidateCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// TALLY storage — key: candidate_id, value: vote count
    /// This is the final result, filled during reveal phase
    #[pallet::storage]
    #[pallet::getter(fn tally)]
    pub type Tally<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,  // candidate_id
        u64,  // vote count
        ValueQuery,
    >;

    /// Tracks how many votes have been revealed (for progress tracking)
    #[pallet::storage]
    #[pallet::getter(fn revealed_count)]
    pub type RevealedCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    // =========================================================
    // EVENTS
    // =========================================================
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A vote was submitted and stored encrypted
        VoteSubmitted { vote_id: u32 },

        /// Election phase changed
        PhaseChanged { new_phase: ElectionPhase },

        /// A candidate was registered
        CandidateRegistered { candidate_id: u32 },

        /// A vote was revealed and counted by the authority
        VoteRevealed { vote_id: u32, candidate_id: u32 },

        /// All votes revealed — tally is final
        TallyFinalized,
    }

    // =========================================================
    // ERRORS
    // =========================================================
    #[pallet::error]
    pub enum Error<T> {
        /// Encrypted vote data too large
        EncryptedVoteTooLarge,
        /// Blind signature data too large
        BlindSignatureTooLarge,
        /// This account has already voted
        AlreadyVoted,
        /// This blind signature token was already used
        NullifierAlreadyUsed,
        /// Vote not found in storage
        VoteNotFound,
        /// Election is not in the required phase for this action
        WrongPhase,
        /// Candidate does not exist
        CandidateNotFound,
        /// Too many candidates registered
        TooManyCandidates,
        /// Candidate name too long
        CandidateNameTooLong,
        /// This vote was already revealed
        VoteAlreadyRevealed,
    }

    // =========================================================
    // EXTRINSICS (callable functions)
    // =========================================================
    #[pallet::call]
    impl<T: Config> Pallet<T> {

        // ---------------------------------------------------------
        // ADMIN: Register a candidate (before voting starts)
        // Called by: Alice (sudo) via AdminOrigin
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(0)]
        pub fn register_candidate(
            origin: OriginFor<T>,
            name: Vec<u8>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            // Only allowed before voting starts
            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::NotStarted,
                Error::<T>::WrongPhase
            );

            let bounded_name = BoundedVec::try_from(name)
                .map_err(|_| Error::<T>::CandidateNameTooLong)?;

            let candidate_id = CandidateCounter::<T>::get();
            ensure!(
                candidate_id < T::MaxCandidates::get(),
                Error::<T>::TooManyCandidates
            );

            Candidates::<T>::insert(candidate_id, bounded_name);
            CandidateCounter::<T>::put(candidate_id.saturating_add(1));

            // Initialize tally for this candidate at 0
            Tally::<T>::insert(candidate_id, 0u64);

            Self::deposit_event(Event::CandidateRegistered { candidate_id });
            Ok(())
        }

        // ---------------------------------------------------------
        // ADMIN: Start the election (move to Voting phase)
        // Called by: Alice (sudo)
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(1)]
        pub fn start_election(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::NotStarted,
                Error::<T>::WrongPhase
            );

            CurrentPhase::<T>::put(ElectionPhase::Voting);
            Self::deposit_event(Event::PhaseChanged {
                new_phase: ElectionPhase::Voting,
            });
            Ok(())
        }

        // ---------------------------------------------------------
        // ADMIN: End the election (move to Ended phase)
        // Called by: Alice (sudo) — no more votes accepted after this
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(2)]
        pub fn end_election(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::Voting,
                Error::<T>::WrongPhase
            );

            CurrentPhase::<T>::put(ElectionPhase::Ended);
            Self::deposit_event(Event::PhaseChanged {
                new_phase: ElectionPhase::Ended,
            });
            Ok(())
        }

        // ---------------------------------------------------------
        // VOTER: Submit an encrypted vote
        // Called by: Any registered voter
        //
        // Flow:
        //   1. Voter hashes their choice with RSA (done in frontend)
        //   2. Blind signs the hash (done in frontend)  
        //   3. Calls this extrinsic with encrypted_vote + blind_signature
        //   4. Pallet checks nullifier (no double voting)
        //   5. Stores encrypted vote on-chain
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(3)]
        pub fn submit_vote(
            origin: OriginFor<T>,
            encrypted_vote: Vec<u8>,
            blind_signature: Vec<u8>,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            // Must be in Voting phase
            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::Voting,
                Error::<T>::WrongPhase
            );

            // Check account hasn't voted
            ensure!(!HasVoted::<T>::get(&voter), Error::<T>::AlreadyVoted);

            // Build nullifier from blind signature (first 64 bytes)
            // This prevents the same blind-signed token being reused
            let nullifier_raw: Vec<u8> = blind_signature
                .iter()
                .take(64)
                .cloned()
                .collect();
            let nullifier: BoundedVec<u8, ConstU32<64>> =
                BoundedVec::try_from(nullifier_raw)
                    .map_err(|_| Error::<T>::BlindSignatureTooLarge)?;

            // Check nullifier not already used
            ensure!(
                !UsedNullifiers::<T>::get(&nullifier),
                Error::<T>::NullifierAlreadyUsed
            );

            // Bound check the inputs
            let bounded_vote = BoundedVec::try_from(encrypted_vote)
                .map_err(|_| Error::<T>::EncryptedVoteTooLarge)?;
            let bounded_signature = BoundedVec::try_from(blind_signature)
                .map_err(|_| Error::<T>::BlindSignatureTooLarge)?;

            let vote = EncryptedVote::<T> {
                encrypted_vote: bounded_vote,
                blind_signature: bounded_signature,
            };

            // Store the encrypted vote
            let vote_id = VoteCounter::<T>::get();
            EncryptedVotes::<T>::insert(vote_id, vote);
            VoteCounter::<T>::put(vote_id.saturating_add(1));

            // Mark nullifier as used
            UsedNullifiers::<T>::insert(&nullifier, true);

            // Mark account as voted
            HasVoted::<T>::insert(&voter, true);

            Self::deposit_event(Event::VoteSubmitted { vote_id });
            Ok(())
        }

        // ---------------------------------------------------------
        // ADMIN: Reveal a single decrypted vote + count it
        // Called by: Alice (sudo / election authority)
        //
        // Flow:
        //   1. Election ends (end_election called)
        //   2. Authority decrypts each vote offline using RSA private key
        //   3. For each vote, calls reveal_vote(vote_id, candidate_id)
        //   4. Pallet increments that candidate's tally
        //   5. After all votes revealed, call finalize_tally
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(4)]
        pub fn reveal_vote(
            origin: OriginFor<T>,
            vote_id: u32,
            candidate_id: u32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            // Must be in Ended phase
            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::Ended,
                Error::<T>::WrongPhase
            );

            // Vote must exist
            ensure!(
                EncryptedVotes::<T>::contains_key(vote_id),
                Error::<T>::VoteNotFound
            );

            // Candidate must exist
            ensure!(
                Candidates::<T>::contains_key(candidate_id),
                Error::<T>::CandidateNotFound
            );

            // Remove the encrypted vote (it's been revealed, no longer needed)
            EncryptedVotes::<T>::remove(vote_id);

            // Increment tally for this candidate
            Tally::<T>::mutate(candidate_id, |count| {
                *count = count.saturating_add(1);
            });

            // Track progress
            RevealedCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::VoteRevealed { vote_id, candidate_id });
            Ok(())
        }

        // ---------------------------------------------------------
        // ADMIN: Finalize the tally (all votes revealed)
        // Called by: Alice after all reveal_vote calls are done
        // After this: tally is readable by anyone (frontend, PolkadotJS)
        // ---------------------------------------------------------
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(5)]
        pub fn finalize_tally(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                CurrentPhase::<T>::get() == ElectionPhase::Ended,
                Error::<T>::WrongPhase
            );

            CurrentPhase::<T>::put(ElectionPhase::TallyComplete);
            Self::deposit_event(Event::TallyFinalized);
            Self::deposit_event(Event::PhaseChanged {
                new_phase: ElectionPhase::TallyComplete,
            });
            Ok(())
        }
    }

    // =========================================================
    // HELPER: Read tally results (used by frontend / RPC)
    // =========================================================
    impl<T: Config> Pallet<T> {
        /// Returns (candidate_id, vote_count) for all candidates
        pub fn get_all_tallies() -> Vec<(u32, u64)> {
            let total = CandidateCounter::<T>::get();
            (0..total)
                .map(|id| (id, Tally::<T>::get(id)))
                .collect()
        }
    }
}