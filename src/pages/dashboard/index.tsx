// FIXED: Cleaned up imports
import { PageLayout } from "@/layouts";

const Dashboard = () => {
  // FIXED: Removed all server fetching logic
  
  return (
    <PageLayout
      title="Dashboard"
      description="Personal AI Workspace"
    >
      <div className="p-4 border rounded-lg bg-muted/20">
        <h3 className="font-medium">Local Mode Active</h3>
        <p className="text-sm text-muted-foreground">
           Running with local API keys. No external connections.
        </p>
      </div>
    </PageLayout>
  );
};

export default Dashboard;