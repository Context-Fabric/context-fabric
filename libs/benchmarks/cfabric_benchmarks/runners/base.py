"""Base benchmark runner interface."""

from __future__ import annotations

from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any, Generic, TypeVar

from cfabric_benchmarks.models.config import BenchmarkConfig


T = TypeVar("T")


class BaseBenchmarkRunner(ABC, Generic[T]):
    """Abstract base class for benchmark runners.

    Provides common functionality for running benchmarks with multiple
    runs and statistical analysis.
    """

    def __init__(self, config: BenchmarkConfig):
        """Initialize benchmark runner.

        Args:
            config: Benchmark configuration
        """
        self.config = config

    @abstractmethod
    def run(self, **kwargs) -> T:
        """Execute the benchmark.

        Returns:
            Benchmark result object
        """
        pass

    @abstractmethod
    def name(self) -> str:
        """Get the name of this benchmark type.

        Returns:
            Benchmark name string
        """
        pass

    def log(self, message: str) -> None:
        """Log a message.

        Args:
            message: Message to log
        """
        print(f"[{self.name()}] {message}", flush=True)


def get_corpus_name(source: str | Path) -> str:
    """Extract corpus name from path.

    Args:
        source: Path to corpus

    Returns:
        Corpus name string
    """
    path = Path(source)
    parts = path.parts

    # Look for common corpus indicators
    for i, part in enumerate(parts):
        if part in ("tf", "text-fabric-data"):
            if i > 0:
                return parts[i - 1].upper()

    return path.parent.name.upper()


def load_tf_api(source: str | Path) -> Any:
    """Load Text-Fabric API for a corpus.

    Args:
        source: Path to TF source files

    Returns:
        Text-Fabric API object
    """
    from tf.fabric import Fabric as TFFabric

    tf = TFFabric(locations=str(source), silent="deep")
    return tf.loadAll(silent="deep")


def load_cf_api(source: str | Path) -> Any:
    """Load Context-Fabric API for a corpus.

    Embedding structures are preloaded automatically by default.
    Set CF_EMBEDDING_CACHE=off environment variable to disable.

    Args:
        source: Path to TF source files

    Returns:
        Context-Fabric API object
    """
    from cfabric import Fabric as CFFabric

    cf = CFFabric(locations=str(source), silent="deep")
    return cf.loadAll(silent="deep")


def get_corpus_stats(api: Any) -> dict:
    """Get statistics about a loaded corpus.

    Args:
        api: Text-Fabric or Context-Fabric API object

    Returns:
        Dictionary with corpus statistics
    """
    return {
        "max_slot": api.F.otype.maxSlot,
        "max_node": api.F.otype.maxNode,
        "node_types": len(api.F.otype.all),
        "node_features": len([f for f in dir(api.F) if not f.startswith("_")]),
        "edge_features": len([f for f in dir(api.E) if not f.startswith("_")]),
    }
