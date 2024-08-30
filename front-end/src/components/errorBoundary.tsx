import React, {
	type ErrorInfo,
	type PropsWithChildren,
	type ReactNode,
} from "react";

// eslint-disable-next-line @typescript-eslint/no-empty-interface
interface ErrorBoundaryProps {}

interface ErrorBoundaryState {
	hasError: boolean;
	error?: unknown;
}

export class ErrorBoundary extends React.Component<
	PropsWithChildren<ErrorBoundaryProps>,
	ErrorBoundaryState
> {
	public constructor(props: ErrorBoundaryProps) {
		super(props);
		this.state = { hasError: false };
	}

	public static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
		return { hasError: true, error };
	}

	public componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
		console.error("Error caught by ErrorBoundary:", error, errorInfo);
		console.log("props:", this.props);
	}

	public render(): ReactNode {
		if (this.state.hasError) {
			return (
				<div className="p-4 bg-red-200 text-red-800">
					<p>Something is not right in here.</p>
				</div>
			);
		}

		return this.props.children;
	}
}
