//+------------------------------------------------------------------+
//| MasterParityOracle.mqh                                           |
//| Optional deterministic input tape for MT5 -> Rust parity proof.  |
//+------------------------------------------------------------------+
#ifndef __KITT_MASTER_PARITY_ORACLE_MQH__
#define __KITT_MASTER_PARITY_ORACLE_MQH__

class CMstParityOracle
{
private:
   bool   m_enabled;
   int    m_frames;
   int    m_levels;
   int    m_nodes;
   int    m_sources;
   int    m_features;
   string m_run_key;
   string m_invocation_id;
   ulong  m_sequence;

   string D(const double value) const { return StringFormat("%.17g", value); }
   bool OpenOne(int &handle, const string path, const string header)
   {
      handle = FileOpen(path, FILE_WRITE|FILE_CSV|FILE_ANSI|FILE_COMMON, '\t');
      if(handle == INVALID_HANDLE) return false;
      FileWriteString(handle, header + "\r\n");
      return true;
   }
   void CloseOne(int &handle)
   {
      if(handle != INVALID_HANDLE) { FileFlush(handle); FileClose(handle); }
      handle = INVALID_HANDLE;
   }

public:
   CMstParityOracle(void)
   {
      m_enabled = false; m_frames = m_levels = m_nodes = m_sources = m_features = INVALID_HANDLE;
      m_run_key = ""; m_invocation_id = ""; m_sequence = 0;
   }

   bool Init(const bool enabled, const string run_key, const string invocation_id)
   {
      Close(); m_enabled = enabled;
      if(!m_enabled) return true;
      m_run_key = run_key; m_invocation_id = invocation_id; m_sequence = 0;
      string root = "MasterStructureParity\\" + run_key + "\\";
      FolderCreate("MasterStructureParity", FILE_COMMON);
      FolderCreate("MasterStructureParity\\" + run_key, FILE_COMMON);
      if(!OpenOne(m_frames, root + "frames.tsv",
         "contract\trun_key\tinvocation_id\tsequence\tmarket_time\tcalculation_bar_time\tclosed_bar_time\tbid\task\treference_price\tclosed_bar_price\tatr\tstructure_snapshot_hash\tstructure_generation\tnode_count\tlevel_count\tsource_count\tauction_event_sequence")) return false;
      if(!OpenOne(m_levels, root + "levels.tsv",
         "contract\trun_key\tsequence\tordinal\tsource_key\tproducer\tproducer_instance\tlocal_id\tfamily\tsource_kind\trole\tlower\tprice\tupper\tnormalized_lower\tnormalized_price\tnormalized_upper\twidth_atr\ttimeframe\tcreated_at\tupdated_at\teffective_start\teffective_end\tdeveloping\tfrozen\tstate\ttouches\trejections\treclaims\tacceptance_bars\tmass\tmass_share\tevidence_weight\tmax_excursion_atr")) return false;
      if(!OpenOne(m_nodes, root + "nodes.tsv",
         "contract\trun_key\tsequence\tordinal\tnode_id\tevidence_id\tcluster_id\tlower\tprice\tupper\tnormalized_lower\tnormalized_price\tnormalized_upper\twidth_atr\tdistance_atr\tregion\tmedian_distance_atr\tmedian_distance_sigma\tcog_distance_sigma\twidth_sigma\tcontains_cog\trole\tfamily_mask\trole_mask\tfamily_count\tmember_count\tdeveloping_count\tfrozen_count\tbroken_count\tcorridor_up_levels\tcorridor_down_levels\tcorridor_up_noise\tcorridor_down_noise\toldest_source_time\tnewest_update_time\tprovenance_offset\tprovenance_count\texistence\tcreated_at\tlast_seen_at\tstate_changed_at\tattempt_count\tinteraction_state\tmissed_rebuilds\trevision\tprimary_parent_id\tsecondary_parent_id")) return false;
      if(!OpenOne(m_sources, root + "sources.tsv",
         "contract\trun_key\tsequence\tordinal\tsource_key\tproducer\tproducer_instance\tlocal_id\tfamily\tsource_kind")) return false;
      if(!OpenOne(m_features, root + "features.tsv",
         "contract\trun_key\tsequence\tfrozen_bar_time\thas_cog\thas_c3\thas_lattice\thas_field\thas_profile\tcog_price\tc3_price\tcog_distance_atr\tc3_distance_atr\tcog_velocity_atr\tc3_velocity_atr\tlattice_width_atr\tfield_width_atr\tprofile_poc\tprofile_vah\tprofile_val\tpoc_distance_atr\tvah_distance_atr\tval_distance_atr\tspread_atr\traw_level_count\tnoise_level_count\tactive_node_count\tregional_valid\tsigma_valid\tregional_has_cog\tbasis_changed\tregional_frozen_bar_time\tstructure_snapshot_hash\tregional_basis_hash\tstructure_generation\tpopulation_count\tpopulation_count_delta\tvelocity_elapsed_bars\tmedian_price\tmean_price\tstructural_sigma\tregional_reference_price\tregional_cog_price\tprice_from_median_atr\tprice_from_median_sigma\tprice_from_cog_atr\tprice_from_cog_sigma\tcog_median_gap_atr\tcog_median_gap_sigma\tcog_velocity_price\tregional_cog_velocity_atr\tcog_velocity_sigma\tmedian_velocity_price\tmedian_velocity_atr\tmedian_velocity_sigma\tsigma_log_change\tprice_region\tcog_region")) return false;
      return true;
   }

   bool Write(const datetime market_time, const datetime closed_bar_time,
              const double reference_price, const double closed_bar_price,
              const double atr, const ulong structure_snapshot_hash,
              const ulong structure_generation, const ulong auction_sequence,
              MST_Level &levels[], const int level_count,
              MST_Node &nodes[], const int node_count,
              MST_SourceRef &sources[], const int source_count,
              const MST_AuctionFeatureSnapshot &feature)
   {
      if(!m_enabled) return true;
      m_sequence++;
      MqlTick tick;
      ZeroMemory(tick);
      SymbolInfoTick(_Symbol, tick);
      FileWrite(m_frames, "MST_AUCTION_REPLAY_INPUT_V1", m_run_key, m_invocation_id,
                m_sequence, (long)market_time, (long)feature.frozen_bar_time,
                (long)closed_bar_time, D(tick.bid), D(tick.ask), D(reference_price),
                D(closed_bar_price), D(atr), structure_snapshot_hash, structure_generation,
                node_count, level_count, source_count, auction_sequence);
      for(int i = 0; i < level_count; i++)
      {
         MST_Level v = levels[i];
         FileWrite(m_levels, "MST_NORMALIZED_STRUCTURE_INPUT_V1", m_run_key, m_sequence, i,
                   v.source_key, (int)v.producer, v.producer_instance, v.local_id,
                   (int)v.family, v.source_kind, (int)v.role, D(v.lower), D(v.price), D(v.upper),
                   D(v.normalized_lower), D(v.normalized_price), D(v.normalized_upper), D(v.width_atr),
                   (int)v.timeframe, (long)v.created_at, (long)v.updated_at, (long)v.effective_start,
                   (long)v.effective_end, (int)v.developing, (int)v.frozen_geometry, (int)v.state,
                   v.touches, v.rejections, v.reclaims, v.acceptance_bars, D(v.mass), D(v.mass_share),
                   D(v.evidence_weight), D(v.max_excursion_atr));
      }
      for(int i = 0; i < node_count; i++)
      {
         MST_Node v = nodes[i];
         FileWrite(m_nodes, "MST_STRUCTURAL_NODE_INPUT_V1", m_run_key, m_sequence, i,
                   v.node_id, v.evidence_id, v.cluster_id, D(v.lower), D(v.price), D(v.upper),
                   D(v.normalized_lower), D(v.normalized_price), D(v.normalized_upper), D(v.width_atr),
                   D(v.distance_atr), (int)v.structural_region, D(v.median_distance_atr),
                   D(v.median_distance_sigma), D(v.cog_distance_sigma), D(v.width_sigma),
                   (int)v.contains_cog, (int)v.role, v.family_mask, v.role_mask, v.family_count,
                   v.member_count, v.developing_count, v.frozen_count, v.broken_count,
                   v.corridor_up_level_count, v.corridor_down_level_count, v.corridor_up_noise_count,
                   v.corridor_down_noise_count, (long)v.oldest_source_time, (long)v.newest_update_time,
                   v.provenance_offset, v.provenance_count, (int)v.existence, (long)v.created_at,
                   (long)v.last_seen_at, (long)v.state_changed_at, v.attempt_count, v.interaction_state,
                   v.missed_rebuilds, v.revision, v.primary_parent_id, v.secondary_parent_id);
      }
      for(int i = 0; i < source_count; i++)
      {
         MST_SourceRef v = sources[i];
         FileWrite(m_sources, "MST_STRUCTURAL_PROVENANCE_INPUT_V1", m_run_key, m_sequence, i,
                   v.source_key, (int)v.producer, v.producer_instance, v.local_id,
                   (int)v.family, v.source_kind);
      }
      MST_RegionalSnapshot r = feature.regional;
      FileWrite(m_features, "MST_CAUSAL_FEATURE_INPUT_V1", m_run_key, m_sequence,
                (long)feature.frozen_bar_time, (int)feature.has_cog, (int)feature.has_c3,
                (int)feature.has_lattice, (int)feature.has_field, (int)feature.has_profile,
                D(feature.cog_price), D(feature.c3_price), D(feature.cog_distance_atr),
                D(feature.c3_distance_atr), D(feature.cog_velocity_atr_per_bar),
                D(feature.c3_velocity_atr_per_bar), D(feature.lattice_width_atr),
                D(feature.field_width_atr), D(feature.profile_poc), D(feature.profile_vah),
                D(feature.profile_val), D(feature.poc_distance_atr), D(feature.vah_distance_atr),
                D(feature.val_distance_atr), D(feature.spread_atr), feature.raw_level_count,
                feature.noise_level_count, feature.active_node_count, (int)r.valid, (int)r.sigma_valid,
                (int)r.has_cog, (int)r.basis_changed, (long)r.frozen_bar_time,
                r.structure_snapshot_hash, r.regional_basis_hash, r.structure_generation,
                r.population_count, r.population_count_delta, r.velocity_elapsed_bars,
                D(r.median_price), D(r.mean_price), D(r.structural_sigma), D(r.reference_price),
                D(r.cog_price), D(r.price_from_median_atr), D(r.price_from_median_sigma),
                D(r.price_from_cog_atr), D(r.price_from_cog_sigma), D(r.cog_median_gap_atr),
                D(r.cog_median_gap_sigma), D(r.cog_velocity_price_per_bar),
                D(r.cog_velocity_atr_per_bar), D(r.cog_velocity_sigma_per_bar),
                D(r.median_velocity_price_per_bar), D(r.median_velocity_atr_per_bar),
                D(r.median_velocity_sigma_per_bar), D(r.sigma_log_change_per_bar),
                (int)r.price_region, (int)r.cog_region);
      return true;
   }

   void Flush(void)
   {
      if(!m_enabled) return;
      FileFlush(m_frames); FileFlush(m_levels); FileFlush(m_nodes);
      FileFlush(m_sources); FileFlush(m_features);
   }

   void Close(void)
   {
      CloseOne(m_frames); CloseOne(m_levels); CloseOne(m_nodes);
      CloseOne(m_sources); CloseOne(m_features);
      m_enabled = false;
   }
};

#endif
