import { motion } from 'framer-motion';
import { Scale, Shield, FileText, Cpu, Coins, Flame, ArrowRight } from 'lucide-react';

const laws = [
  {
    icon: <Scale size={24} color="var(--cyan)" />,
    title: "Every flow has an owner",
    desc: "Aucun flux fiat, crypto, compute, wallet, smart contract, IP, taxe ou burn ne doit exister sans responsable identifié."
  },
  {
    icon: <Shield size={24} color="var(--green)" />,
    title: "Every owner has duties",
    desc: "Posséder une ressource CYNIC implique des devoirs : uptime, comptabilité, sécurité, reporting, fiscalité, maintenance ou gouvernance."
  },
  {
    icon: <FileText size={24} color="var(--amber)" />,
    title: "Every duty has receipts",
    desc: "Les devoirs critiques doivent produire des preuves primaires : factures, transactions on-chain, audits. Le Kernel indexe ces preuves."
  },
  {
    icon: <Cpu size={24} color="var(--text)" />,
    title: "Survival before burn",
    desc: "Aucune branche ne doit brûler ou redistribuer ce qui est nécessaire à ses coûts directs, ses impôts, sa sécurité et sa durabilité."
  },
  {
    icon: <Coins size={24} color="var(--dim)" />,
    title: "Token is not equity",
    desc: "Le Token ne représente pas une action de TALARIA SAS, ne donne pas droit aux dividendes légaux, et ne constitue pas une créance sur les actifs."
  },
  {
    icon: <Flame size={24} color="var(--red)" />,
    title: "Rewards follow verified contribution",
    desc: "Les revenus on-chain rémunèrent le travail, le risque, la liquidité ou la sécurité vérifiés ; ils ne rémunèrent pas la simple proximité sociale."
  }
];

export default function Manifest() {
  const containerVariants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: { staggerChildren: 0.1 }
    }
  };

  const itemVariants = {
    hidden: { opacity: 0, y: 20 },
    show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: "easeOut" } }
  };

  return (
    <motion.section 
      className="section"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.8 }}
    >
      <div className="section-heading" style={{ maxWidth: '900px' }}>
        <p className="eyebrow" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Scale size={16} /> TALARIA SAS Genesis
        </p>
        <h2>CYNIC Ownership Constitution</h2>
        <p className="lead" style={{ marginTop: '1.5rem', color: 'var(--text)', fontWeight: 300 }}>
          "La souveraineté ne vient pas de l'absence de propriétaire. Elle vient de propriétaires identifiés, responsables, mesurés et limités par des règles publiques."
        </p>
        <p className="section-copy" style={{ marginTop: '1rem' }}>
          Ce manifeste formalise le principe d'ownership. Il sépare clairement les propriétaires légaux, les opérateurs, les smart contracts, les réserves fiscales et les preuves publiques.
        </p>
      </div>

      <motion.div 
        className="sas-grid" 
        style={{ marginTop: '4rem' }}
        variants={containerVariants}
        initial="hidden"
        animate="show"
      >
        {laws.map((law, idx) => (
          <motion.article 
            key={idx}
            variants={itemVariants}
            className="rail-detail"
            style={{ display: 'flex', flexDirection: 'column', gap: '1rem', minHeight: 'auto' }}
            whileHover={{ y: -4, borderColor: 'var(--border)', background: 'var(--surface-strong)' }}
          >
            <div style={{ background: 'rgba(255,255,255,0.03)', padding: '12px', borderRadius: '8px', display: 'inline-block', alignSelf: 'flex-start' }}>
              {law.icon}
            </div>
            <h3 style={{ margin: '8px 0 0 0', fontSize: '18px' }}>{law.title}</h3>
            <p style={{ color: 'var(--muted)', fontSize: '0.95rem', lineHeight: '1.6', flexGrow: 1 }}>
              {law.desc}
            </p>
          </motion.article>
        ))}
      </motion.div>

      <motion.div 
        style={{ marginTop: '5rem', borderTop: '1px solid var(--border)', paddingTop: '4rem' }}
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
      >
        <h3>Protocol Revenue Waterfall</h3>
        <p className="section-copy" style={{ marginBottom: '2rem' }}>
          L'ordre d'exécution est intentionnel. Le burn ou les rewards ne doivent jamais passer avant les coûts de survie de l'organisme.
        </p>
        
        <div style={{ 
          background: 'var(--surface)', 
          border: '1px solid var(--border)', 
          borderRadius: '8px', 
          padding: '2rem',
          fontFamily: 'monospace',
          color: 'var(--cyan)'
        }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ color: 'var(--text)', fontWeight: 'bold' }}>100% Branch Protocol Revenue</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem' }}><ArrowRight size={16} color="var(--dim)"/> Direct costs reserve</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem' }}><ArrowRight size={16} color="var(--dim)"/> Tax & accounting reserve</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem' }}><ArrowRight size={16} color="var(--dim)"/> Operator payments</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem' }}><ArrowRight size={16} color="var(--dim)"/> TALARIA SAS IP / governance fee</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem' }}><ArrowRight size={16} color="var(--dim)"/> Security & audit reserve</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '2rem', color: 'var(--green)' }}><ArrowRight size={16} color="var(--green)"/> <strong>Protocol Surplus</strong></div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '4rem', color: 'var(--muted)' }}><ArrowRight size={16}/> Buyback / Burn</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '4rem', color: 'var(--muted)' }}><ArrowRight size={16}/> Grants & Liquidity incentives</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', paddingLeft: '4rem', color: 'var(--muted)' }}><ArrowRight size={16}/> E-Score rewards</div>
          </div>
        </div>
      </motion.div>
    </motion.section>
  );
}
