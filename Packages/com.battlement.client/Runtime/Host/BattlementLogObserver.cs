#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    internal sealed class BattlementLogObserver : IDisposable
    {
        private const int MaximumRecords = 2_048;
        private readonly object gate = new();
        private readonly Queue<BattlementLogEntry> records = new();
        private readonly Action<BattlementLogObserver> unregister;
        private bool disposed;
        private bool overflowed;

        public BattlementLogObserver(Action<BattlementLogObserver> unregister) =>
            this.unregister = unregister;

        public bool Overflowed
        {
            get
            {
                lock (gate)
                {
                    return overflowed;
                }
            }
        }

        public int Count
        {
            get
            {
                lock (gate)
                {
                    return records.Count;
                }
            }
        }

        public BattlementLogEntry[] Drain()
        {
            lock (gate)
            {
                BattlementLogEntry[] result = records.ToArray();
                records.Clear();
                return result;
            }
        }

        public void Dispose()
        {
            unregister(this);
            lock (gate)
            {
                disposed = true;
            }
        }

        internal void Accept(BattlementLogEntry entry)
        {
            lock (gate)
            {
                if (disposed || overflowed)
                {
                    return;
                }
                if (records.Count == MaximumRecords)
                {
                    overflowed = true;
                    return;
                }

                records.Enqueue(entry);
            }
        }
    }
}
